use anyhow::{bail, Context, Result};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const SESSION_SPEC: &str = "rms/verification-session/v0.1";
const LOCK_SPEC: &str = "rms/verification-lock/v0.1";

#[derive(Clone, Debug, Serialize, Deserialize)]
struct LockRecord {
    spec: String,
    identity: String,
    kind: String,
    pid: u32,
    started_at_unix_ms: u128,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct SessionRecord {
    spec: String,
    identity: String,
    kind: String,
    source_revision: String,
    input_digest: String,
    tool_digest: String,
    seed: Option<u64>,
    started_at_unix_ms: u128,
    updated_at_unix_ms: u128,
    completed: BTreeMap<String, CompletedPhase>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct CompletedPhase {
    label: String,
    elapsed_ms: u128,
    completed_at_unix_ms: u128,
}

pub(super) struct VerificationSession {
    identity: String,
    total: usize,
    next: usize,
    record: SessionRecord,
    record_path: PathBuf,
    lock_path: PathBuf,
    run_started: Instant,
}

pub(super) struct ProofCache {
    root: PathBuf,
}

impl ProofCache {
    pub(super) fn new(
        root: &Path,
        source_digest: &str,
        tool_digest: &str,
        seed: Option<u64>,
    ) -> Result<Self> {
        let identity = verification_identity(
            "proof",
            "content-addressed",
            source_digest,
            tool_digest,
            seed,
        );
        let root = root.join(".rms/cache/verification/proofs").join(identity);
        fs::create_dir_all(&root)?;
        Ok(Self { root })
    }

    pub(super) fn key(parts: &[&str]) -> String {
        let mut digest = Sha256::new();
        for part in parts {
            digest.update(part.as_bytes());
            digest.update([0]);
        }
        format!("{:x}", digest.finalize())
    }

    pub(super) fn load<T: DeserializeOwned>(&self, key: &str) -> Option<T> {
        let source = fs::read_to_string(self.root.join(format!("{key}.json"))).ok()?;
        serde_json::from_str(&source).ok()
    }

    pub(super) fn store<T: Serialize>(&self, key: &str, value: &T) -> Result<()> {
        write_json_atomic(&self.root.join(format!("{key}.json")), value)
    }
}

impl VerificationSession {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn acquire(
        root: &Path,
        kind: &str,
        source_revision: &str,
        input_digest: &str,
        tool_digest: &str,
        seed: Option<u64>,
        phases: usize,
    ) -> Result<Self> {
        let identity =
            verification_identity(kind, source_revision, input_digest, tool_digest, seed);
        let cache = root.join(".rms/cache/verification");
        fs::create_dir_all(&cache).with_context(|| {
            format!("failed to create verification cache `{}`", cache.display())
        })?;
        let lock_path = cache.join(format!("{kind}-{identity}.lock.json"));
        acquire_lock(&lock_path, kind, &identity)?;
        let record_path = cache.join(format!("{kind}-{identity}.json"));
        let now = now_ms();
        let record = read_session(&record_path)
            .filter(|record| {
                record.spec == SESSION_SPEC
                    && record.identity == identity
                    && record.kind == kind
                    && record.source_revision == source_revision
                    && record.input_digest == input_digest
                    && record.tool_digest == tool_digest
                    && record.seed == seed
            })
            .unwrap_or_else(|| SessionRecord {
                spec: SESSION_SPEC.to_string(),
                identity: identity.clone(),
                kind: kind.to_string(),
                source_revision: source_revision.to_string(),
                input_digest: input_digest.to_string(),
                tool_digest: tool_digest.to_string(),
                seed,
                started_at_unix_ms: now,
                updated_at_unix_ms: now,
                completed: BTreeMap::new(),
            });
        write_json_atomic(&record_path, &record)?;
        Ok(Self {
            identity,
            total: phases.max(1),
            next: 0,
            record,
            record_path,
            lock_path,
            run_started: Instant::now(),
        })
    }

    pub(super) fn identity(&self) -> &str {
        &self.identity
    }

    pub(super) fn run_phase<F>(&mut self, id: &str, label: &str, action: F) -> Result<()>
    where
        F: FnOnce() -> Result<()>,
    {
        self.next += 1;
        let position = self.next.min(self.total);
        let elapsed = self.run_started.elapsed();
        if let Some(completed) = self.record.completed.get(id) {
            eprintln!(
                "verify [{position}/{}] phase={id} runner={} elapsed={} eta={} state=resume-cache cached={}ms",
                self.total,
                shell_word(label),
                duration_label(elapsed),
                eta_class(position, self.total, elapsed),
                completed.elapsed_ms
            );
            return Ok(());
        }
        eprintln!(
            "verify [{position}/{}] phase={id} runner={} elapsed={} eta={} state=starting",
            self.total,
            shell_word(label),
            duration_label(elapsed),
            eta_class(position.saturating_sub(1), self.total, elapsed)
        );
        let started = Instant::now();
        let result = action();
        let phase_elapsed = started.elapsed();
        match result {
            Ok(()) => {
                let now = now_ms();
                self.record.completed.insert(
                    id.to_string(),
                    CompletedPhase {
                        label: label.to_string(),
                        elapsed_ms: phase_elapsed.as_millis(),
                        completed_at_unix_ms: now,
                    },
                );
                self.record.updated_at_unix_ms = now;
                write_json_atomic(&self.record_path, &self.record)?;
                eprintln!(
                    "verify [{position}/{}] phase={id} runner={} elapsed={} eta={} state=complete",
                    self.total,
                    shell_word(label),
                    duration_label(self.run_started.elapsed()),
                    eta_class(position, self.total, self.run_started.elapsed())
                );
                Ok(())
            }
            Err(error) => {
                eprintln!(
                    "verify [{position}/{}] phase={id} runner={} elapsed={} eta=blocked state=failed",
                    self.total,
                    shell_word(label),
                    duration_label(self.run_started.elapsed())
                );
                Err(error)
            }
        }
    }
}

impl Drop for VerificationSession {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.lock_path);
    }
}

fn acquire_lock(path: &Path, kind: &str, identity: &str) -> Result<()> {
    let record = LockRecord {
        spec: LOCK_SPEC.to_string(),
        identity: identity.to_string(),
        kind: kind.to_string(),
        pid: std::process::id(),
        started_at_unix_ms: now_ms(),
    };
    for _ in 0..2 {
        match OpenOptions::new().create_new(true).write(true).open(path) {
            Ok(mut file) => {
                serde_json::to_writer_pretty(&mut file, &record)?;
                file.write_all(b"\n")?;
                file.sync_all()?;
                return Ok(());
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                let existing = fs::read_to_string(path)
                    .ok()
                    .and_then(|source| serde_json::from_str::<LockRecord>(&source).ok());
                if let Some(existing) = existing.filter(|existing| process_is_alive(existing.pid)) {
                    bail!(
                        "duplicate {kind} verification `{identity}` is already active in process {}; state=verification-lock-blocked (this process did not wait on a package lock)",
                        existing.pid
                    );
                }
                fs::remove_file(path).with_context(|| {
                    format!(
                        "failed to remove stale verification lock `{}`",
                        path.display()
                    )
                })?;
            }
            Err(error) => {
                return Err(error).with_context(|| {
                    format!("failed to acquire verification lock `{}`", path.display())
                })
            }
        }
    }
    bail!("verification lock acquisition raced for `{identity}`; retry the command")
}

fn process_is_alive(pid: u32) -> bool {
    #[cfg(unix)]
    {
        return Command::new("kill")
            .args(["-0", &pid.to_string()])
            .status()
            .is_ok_and(|status| status.success());
    }
    #[cfg(not(unix))]
    {
        let _ = pid;
        false
    }
}

fn read_session(path: &Path) -> Option<SessionRecord> {
    let source = fs::read_to_string(path).ok()?;
    serde_json::from_str(&source).ok()
}

fn write_json_atomic(path: &Path, value: &impl Serialize) -> Result<()> {
    let temporary = path.with_extension(format!("tmp-{}", std::process::id()));
    let mut bytes = serde_json::to_vec_pretty(value)?;
    bytes.push(b'\n');
    fs::write(&temporary, bytes)
        .with_context(|| format!("failed to write `{}`", temporary.display()))?;
    fs::rename(&temporary, path).with_context(|| {
        format!(
            "failed to replace verification session `{}` with `{}`",
            path.display(),
            temporary.display()
        )
    })
}

pub(super) fn verification_identity(
    kind: &str,
    source_revision: &str,
    input_digest: &str,
    tool_digest: &str,
    seed: Option<u64>,
) -> String {
    let mut digest = Sha256::new();
    for part in [kind, source_revision, input_digest, tool_digest] {
        digest.update(part.as_bytes());
        digest.update([0]);
    }
    digest.update(seed.unwrap_or_default().to_le_bytes());
    format!("{:x}", digest.finalize())[..16].to_string()
}

pub(super) fn eta_class(completed: usize, total: usize, elapsed: Duration) -> &'static str {
    if completed == 0 || total <= completed {
        return if total <= completed {
            "complete"
        } else {
            "estimating"
        };
    }
    let remaining = total.saturating_sub(completed) as u128;
    let estimate_ms = elapsed.as_millis().saturating_mul(remaining) / completed as u128;
    match estimate_ms {
        0..=29_999 => "under-30s",
        30_000..=119_999 => "under-2m",
        120_000..=599_999 => "minutes",
        _ => "long",
    }
}

fn duration_label(duration: Duration) -> String {
    let seconds = duration.as_secs();
    if seconds < 60 {
        format!("{seconds}s")
    } else {
        format!("{}m{}s", seconds / 60, seconds % 60)
    }
}

fn shell_word(value: &str) -> String {
    if value.chars().all(|character| {
        character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.' | '/' | ':')
    }) {
        value.to_string()
    } else {
        format!("{:?}", value)
    }
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "rms-verification-{label}-{}-{}",
            std::process::id(),
            now_ms()
        ))
    }

    #[test]
    fn same_identity_is_locked_and_completed_phases_resume() {
        let root = temp_root("lock-resume");
        fs::create_dir_all(&root).unwrap();
        let mut first =
            VerificationSession::acquire(&root, "release-check", "abc", "input", "tools", None, 2)
                .unwrap();
        first.run_phase("one", "runner one", || Ok(())).unwrap();
        let duplicate =
            VerificationSession::acquire(&root, "release-check", "abc", "input", "tools", None, 2)
                .err()
                .unwrap();
        assert!(duplicate.to_string().contains("verification-lock-blocked"));
        drop(first);

        let mut resumed =
            VerificationSession::acquire(&root, "release-check", "abc", "input", "tools", None, 2)
                .unwrap();
        let mut reran = false;
        resumed
            .run_phase("one", "runner one", || {
                reran = true;
                Ok(())
            })
            .unwrap();
        assert!(!reran);
        resumed.run_phase("two", "runner two", || Ok(())).unwrap();
        drop(resumed);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn identity_and_eta_change_only_with_relevant_inputs() {
        let first = verification_identity("release", "rev", "input", "tools", Some(1));
        assert_eq!(
            first,
            verification_identity("release", "rev", "input", "tools", Some(1))
        );
        assert_ne!(
            first,
            verification_identity("release", "rev", "changed", "tools", Some(1))
        );
        assert_eq!(eta_class(0, 10, Duration::from_secs(1)), "estimating");
        assert_eq!(eta_class(10, 10, Duration::from_secs(1)), "complete");
        assert_eq!(eta_class(1, 2, Duration::from_secs(10)), "under-30s");
    }

    #[test]
    fn proof_cache_reuses_only_the_exact_source_tool_and_seed_identity() {
        let root = temp_root("proof-cache");
        fs::create_dir_all(&root).unwrap();
        let first = ProofCache::new(&root, "source", "tools", Some(7)).unwrap();
        let key = ProofCache::key(&["property", "runner"]);
        first.store(&key, &vec!["pass"]).unwrap();
        assert_eq!(
            first.load::<Vec<String>>(&key),
            Some(vec!["pass".to_string()])
        );

        let changed_source = ProofCache::new(&root, "changed", "tools", Some(7)).unwrap();
        let changed_tools = ProofCache::new(&root, "source", "changed", Some(7)).unwrap();
        let changed_seed = ProofCache::new(&root, "source", "tools", Some(8)).unwrap();
        assert!(changed_source.load::<Vec<String>>(&key).is_none());
        assert!(changed_tools.load::<Vec<String>>(&key).is_none());
        assert!(changed_seed.load::<Vec<String>>(&key).is_none());
        fs::remove_dir_all(root).unwrap();
    }
}
