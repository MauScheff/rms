//! Leaf retirement: observations -> sealed plan -> exact directory rename.
//! Archives are historical provenance, never active implementation proof.
use super::*;

const PLAN_SPEC: &str = "rms/module-retirement-plan/v0.1";
const RECORD_SPEC: &str = "rms/module-retirement/v0.1";
const STORE: &str = ".rms/retirements";

#[derive(Subcommand)]
pub enum RetirementCommand {
    /// Inspect an exact leaf and issue a fresh retirement-only receipt.
    Plan {
        module: PathBuf,
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long)]
        reason: String,
    },
    /// Archive the exact sealed inventory. Dry-run performs no writes.
    Apply {
        module: PathBuf,
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long)]
        route_receipt: PathBuf,
        #[arg(long)]
        dry_run: bool,
    },
    /// Validate retirement provenance and detect unrecorded module deletion.
    Check {
        #[arg(long, default_value = ".")]
        root: PathBuf,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct FileProof {
    sha256: String,
    executable: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Plan {
    spec: String,
    repository: String,
    base: String,
    module: String,
    manifest: String,
    reason: String,
    files: BTreeMap<String, FileProof>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Record {
    spec: String,
    plan: Plan,
    receipt: RouteReceipt,
}

fn plan_id(plan: &Plan) -> Result<String> {
    Ok(sha256_bytes(&serde_json::to_vec(plan)?))
}

fn task(plan: &Plan) -> Result<String> {
    Ok(format!(
        "Retire exact leaf `{}`: {}. Sealed retirement plan {}",
        plan.manifest,
        plan.reason,
        plan_id(plan)?
    ))
}

fn git(root: &Path, args: &[&str]) -> Result<Vec<u8>> {
    let output = Command::new("git").current_dir(root).args(args).output()?;
    if !output.status.success() {
        bail!(
            "retirement Git inspection failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(output.stdout)
}

fn safe_path(value: &str) -> Result<&Path> {
    let path = Path::new(value);
    if value.is_empty()
        || value.chars().any(|c| c.is_control() || c == '\\')
        || value.split('/').any(|part| matches!(part, "" | "." | ".."))
        || !path.components().all(|c| matches!(c, Component::Normal(_)))
    {
        bail!("retirement path must be an exact safe relative path: `{value}`");
    }
    Ok(path)
}

fn manifest_directory(value: &str) -> Result<&Path> {
    let path = safe_path(value)?;
    if path.file_name().and_then(|n| n.to_str()) != Some("module.yaml")
        || path.components().count() < 2
        || matches!(path.components().next(), Some(Component::Normal(n)) if n == ".rms" || n == ".git")
    {
        bail!("retirement requires a non-root active directory with exact module.yaml");
    }
    Ok(path.parent().unwrap())
}

// Check each component even when the final path does not exist.
fn no_symlinks(root: &Path, relative: &Path) -> Result<()> {
    let mut path = root.to_path_buf();
    for component in relative.components() {
        path.push(component);
        match fs::symlink_metadata(&path) {
            Ok(meta) if meta.file_type().is_symlink() => {
                bail!("retirement refuses symlink `{}`", path.display())
            }
            Ok(_) => (),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
            Err(e) => return Err(e.into()),
        }
    }
    Ok(())
}

fn filesystem_inventory(root: &Path) -> Result<BTreeMap<String, FileProof>> {
    let mut files = BTreeMap::new();
    for entry in WalkDir::new(root).follow_links(false) {
        let entry = entry?;
        let meta = fs::symlink_metadata(entry.path())?;
        if meta.file_type().is_symlink() || (!meta.is_dir() && !meta.is_file()) {
            bail!(
                "retirement refuses non-regular path `{}`",
                entry.path().display()
            );
        }
        if meta.is_dir() {
            continue;
        }
        #[cfg(unix)]
        let executable = {
            use std::os::unix::fs::MetadataExt;
            if meta.nlink() != 1 {
                bail!(
                    "retirement refuses shared hard-linked file `{}`",
                    entry.path().display()
                );
            }
            meta.mode() & 0o111 != 0
        };
        #[cfg(not(unix))]
        let executable = false;
        let relative = entry
            .path()
            .strip_prefix(root)?
            .to_str()
            .context("retirement requires UTF-8 paths")?
            .to_string();
        safe_path(&relative)?;
        files.insert(
            relative,
            FileProof {
                sha256: sha256_bytes(&fs::read(entry.path())?),
                executable,
            },
        );
    }
    Ok(files)
}

fn git_inventory(root: &Path, base: &str, directory: &Path) -> Result<BTreeMap<String, FileProof>> {
    if base.len() != 40 || !base.bytes().all(|b| b.is_ascii_hexdigit()) {
        bail!("invalid retirement base commit");
    }
    let directory = directory.to_str().context("non-UTF-8 directory")?;
    let bytes = git(root, &["ls-tree", "-rz", base, "--", directory])?;
    let mut files = BTreeMap::new();
    for line in bytes.split(|b| *b == 0).filter(|s| !s.is_empty()) {
        let line = std::str::from_utf8(line)?;
        let (meta, path) = line.split_once('\t').context("invalid Git tree entry")?;
        let fields = meta.split_whitespace().collect::<Vec<_>>();
        if fields.len() != 3 || !matches!(fields[0], "100644" | "100755") || fields[1] != "blob" {
            bail!("retirement refuses symlink, submodule, or unsupported Git mode at `{path}`");
        }
        let relative = Path::new(path)
            .strip_prefix(directory)?
            .to_str()
            .context("non-UTF-8 file")?
            .to_string();
        safe_path(&relative)?;
        files.insert(
            relative,
            FileProof {
                sha256: sha256_bytes(&git(root, &["cat-file", "blob", fields[2]])?),
                executable: fields[0] == "100755",
            },
        );
    }
    if !files.contains_key("module.yaml") {
        bail!("retirement target has no tracked module.yaml at base");
    }
    Ok(files)
}

fn has_content(value: &YamlValue) -> bool {
    match value {
        YamlValue::Null => false,
        YamlValue::Sequence(v) => !v.is_empty(),
        YamlValue::Mapping(v) => v.values().any(has_content),
        YamlValue::String(s) => !s.is_empty(),
        _ => true,
    }
}

fn leaf_policy(value: &YamlValue) -> Result<String> {
    if get_str(value, &["spec"]) != Some("rms/module/v0.1") {
        bail!("retirement target is not an RMS leaf module");
    }
    for field in [
        "owns",
        "provides",
        "requires",
        "effects",
        "invariants",
        "composition",
        "protocols",
        "resources",
        "artifacts",
        "transformations",
    ] {
        if get_path(value, &[field]).is_some_and(has_content) {
            bail!("leaf retirement refuses nonempty `{field}`; resolve semantic ownership and consumers explicitly first");
        }
    }
    Ok(get_str(value, &["module", "name"])
        .filter(|n| !n.trim().is_empty())
        .context("retirement target requires a module name")?
        .to_string())
}

// Conservative static reference screening is a refusal aid, not proof of dynamic non-use.
// The exact reason is the caller's explicit retirement decision; no inferred ownership.
fn references(root: &Path, directory: &Path, module: &str) -> Result<()> {
    let tracked = git(
        root,
        &[
            "ls-files",
            "-z",
            "--cached",
            "--others",
            "--exclude-standard",
        ],
    )?;
    for raw in tracked.split(|b| *b == 0).filter(|s| !s.is_empty()) {
        let name = std::str::from_utf8(raw)?;
        let path = safe_path(name)?;
        if path.starts_with(".rms/retirements")
            || path.starts_with(".rms/runs")
            || path.starts_with(".rms/cache")
        {
            continue;
        }
        // Prose and immutable historical evidence are not active consumers.
        if path.extension().is_some_and(|e| e == "md") {
            continue;
        }
        let absolute = root.join(path);
        if !absolute.is_file() {
            continue;
        }
        no_symlinks(root, path)?;
        let bytes = fs::read(&absolute)?;
        let Ok(text) = std::str::from_utf8(&bytes) else {
            continue;
        };
        screen_reference(path, text, directory, module)?;
    }
    Ok(())
}

// Pure refusal policy over a source observation; used for live and archived files.
fn screen_reference(path: &Path, text: &str, directory: &Path, module: &str) -> Result<()> {
    if path.extension().is_some_and(|e| e == "md") {
        return Ok(());
    }
    let inside = path.starts_with(directory);
    let name = path.display();
    if !inside && (text.contains(module) || text.contains(directory.to_str().unwrap())) {
        bail!("retirement has an external reference in `{name}`; resolve the consumer before retirement");
    }
    if inside
        && path != directory.join("module.yaml")
        && matches!(
            path.extension().and_then(|s| s.to_str()),
            Some("yaml" | "yml")
        )
    {
        let value: YamlValue = serde_yaml::from_str(text)?;
        if matches!(
            get_str(&value, &["spec"]),
            Some("rms/module/v0.1" | "rms/composite-module/v0.1")
        ) {
            bail!("retirement refuses a nested module at `{name}`");
        }
        if get_str(&value, &["spec"]).is_some_and(|s| s.starts_with("rms/implementation/")) {
            if get_str(&value, &["source", "root"]) != Some(".") {
                bail!("leaf retirement requires an exact local source.root `.` in `{name}`");
            }
        }
    }
    for token in text.split(|c: char| {
        c.is_whitespace() || matches!(c, '\'' | '"' | '`' | '(' | ')' | '[' | ']' | ',' | ';')
    }) {
        let token = token.split('#').next().unwrap_or(token);
        if !token.starts_with('.') && !token.starts_with('/') {
            continue;
        }
        if token == "."
            || token == "..."
            || token == "/"
            || token.starts_with("//")
            || token.contains("${")
        {
            continue;
        }
        let mut resolved = PathBuf::new();
        for component in path
            .parent()
            .unwrap_or(Path::new(""))
            .join(token)
            .components()
        {
            match component {
                Component::CurDir => (),
                Component::ParentDir => {
                    if !resolved.pop() {
                        resolved.push("..");
                    }
                }
                other => resolved.push(other.as_os_str()),
            }
        }
        if inside
            && (token.starts_with('/')
                || (token.starts_with("../") && !resolved.starts_with(directory)))
        {
            bail!("retirement refuses an external or ambiguous source path `{token}` in `{name}`");
        }
        if !inside && resolved.starts_with(directory) {
            bail!("retirement has a relative consumer path `{token}` in `{name}`");
        }
    }
    Ok(())
}

fn observe(root: &Path, manifest: &str, reason: &str) -> Result<Plan> {
    let directory = manifest_directory(manifest)?;
    no_symlinks(root, directory)?;
    no_symlinks(root, Path::new(STORE))?;
    if reason.trim().is_empty() {
        bail!("retirement requires an explicit nonblank reason");
    }
    if git_repository_identity(root)? != root.to_string_lossy() {
        bail!("--root must be the exact Git worktree root");
    }
    if !git(root, &["status", "--porcelain", "--untracked-files=all"])?.is_empty() {
        bail!("retirement plan/apply requires a clean tracked worktree and no untracked files; commit the intended baseline first");
    }
    validate(root)?;
    let base = git_head(root)?;
    let files = filesystem_inventory(&root.join(directory))?;
    if files != git_inventory(root, &base, directory)? {
        bail!("retirement inventory differs from committed files/modes (including ignored or shared files)");
    }
    let value: YamlValue = serde_yaml::from_slice(&fs::read(root.join(manifest))?)?;
    let module = leaf_policy(&value)?;
    references(root, directory, &module)?;
    Ok(Plan {
        spec: PLAN_SPEC.into(),
        repository: git_repository_identity(root)?,
        base,
        module,
        manifest: manifest.into(),
        reason: reason.into(),
        files,
    })
}

fn plan(root: &Path, manifest: &str, reason: &str) -> Result<JsonValue> {
    let plan = observe(root, manifest, reason)?;
    let task = task(&plan)?;
    let run_dir = create_route_run_record(
        root,
        "retire-module",
        &task,
        "Explicit exact-leaf retirement; no provider inference or semantic repair.",
        None,
    )?;
    let target = root.join(manifest);
    let receipt = issue_route_receipt(
        root,
        &run_dir,
        &task,
        None,
        "ready",
        "module-retirement",
        Some(&target),
        None,
        vec!["retire-module".into()],
        vec![target.clone()],
        None,
        None,
    )?;
    fs::write(
        run_dir.join("retirement-plan.json"),
        serde_json::to_vec_pretty(&plan)?,
    )?;
    Ok(
        json!({"result":"ready", "plan":plan, "route_receipt":receipt.receipt_path, "archive":format!("{STORE}/{}/archive", plan_id(&plan)?), "scope":"retirement provenance only; surviving native paths require project-native proof"}),
    )
}

fn apply(root: &Path, manifest: &str, reference: &Path, dry_run: bool) -> Result<JsonValue> {
    let receipt =
        validate_route_receipt(root, reference, "retire-module", &root.join(manifest), None)?;
    let receipt_path = resolve_route_receipt_path(root, reference)?;
    let plan: Plan = serde_json::from_slice(&fs::read(
        receipt_path.parent().unwrap().join("retirement-plan.json"),
    )?)?;
    validate_seal(&plan, &receipt)?;
    if plan.manifest != manifest || observe(root, manifest, &plan.reason)? != plan {
        bail!("retirement observations changed; issue a fresh plan");
    }
    let id = plan_id(&plan)?;
    let destination = root.join(STORE).join(&id);
    if destination.exists() {
        bail!("retirement destination already exists; no overwrite is allowed");
    }
    if dry_run {
        return Ok(
            json!({"result":"ready", "dry_run":true, "retirement_id":id, "scope":"exact directory archive; no files changed"}),
        );
    }
    let record = Record {
        spec: RECORD_SPEC.into(),
        plan,
        receipt,
    };
    fs::create_dir_all(root.join(STORE))?;
    fs::create_dir(&destination)?;
    // Write the durable intent before moving anything. An interrupted pending record
    // fails validation and contains enough information for an explicit restoration.
    let pending = destination.join("pending.json");
    fs::write(&pending, serde_json::to_vec_pretty(&record)?)?;
    fs::File::open(&pending)?.sync_all()?;
    let source = root.join(manifest_directory(manifest)?);
    fs::rename(&source, destination.join("archive"))?;
    fs::rename(&pending, destination.join("retirement.json"))?;
    // No automatic delete or rollback can erase evidence after an interrupted move.
    validate(root)?;
    Ok(
        json!({"result":"applied", "retirement_id":id, "record":destination.join("retirement.json"), "scope":"historical provenance only; native acceptance and authorized commit remain required"}),
    )
}

fn validate_seal(plan: &Plan, receipt: &RouteReceipt) -> Result<()> {
    manifest_directory(&plan.manifest)?;
    let target = Path::new(&plan.repository)
        .join(&plan.manifest)
        .display()
        .to_string();
    if plan.spec != PLAN_SPEC
        || plan.reason.trim().is_empty()
        || receipt.payload.schema != ROUTE_RECEIPT_SPEC
        || receipt.receipt_id != sha256_bytes(&serde_json::to_vec(&receipt.payload)?)
        || receipt.payload.task_sha256 != sha256_bytes(task(plan)?.as_bytes())
        || receipt.payload.route_result != "ready"
        || receipt.payload.lane != "module-retirement"
        || receipt.payload.allowed_action_families != ["retire-module"]
        || receipt.payload.normalized_target_paths != [target.clone()]
        || receipt.payload.owner_module.as_deref() != Some(&target)
        || receipt.payload.implementation_target.is_some()
        || receipt.payload.scaffold.is_some()
        || receipt.payload.repair_authority.is_some()
        || receipt.payload.repository != plan.repository
        || receipt.payload.git_head != plan.base
    {
        bail!("invalid retirement plan/receipt seal or authority");
    }
    Ok(())
}

// Discover deleted RMS manifests independently of current active discovery. History
// remains necessary after the deletion commit and after removal of its archive record.
fn absent_manifests(root: &Path) -> Result<BTreeSet<String>> {
    let mut paths = BTreeSet::new();
    let mut bytes = git(
        root,
        &[
            "log",
            "-m",
            "--format=",
            "--name-only",
            "--diff-filter=D",
            "--no-renames",
            "HEAD",
            "--",
            "*.yaml",
            "*.yml",
        ],
    )?;
    bytes.extend(git(
        root,
        &[
            "diff",
            "--name-only",
            "--diff-filter=D",
            "--no-renames",
            "HEAD",
            "--",
            "*.yaml",
            "*.yml",
        ],
    )?);
    for name in std::str::from_utf8(&bytes)?
        .lines()
        .filter(|s| !s.is_empty())
    {
        if name.starts_with(".rms/") || root.join(name).exists() {
            continue;
        }
        safe_path(name)?;
        if git_yaml_at_revision(root, "HEAD", name)
            .is_some_and(|v| get_str(&v, &["spec"]) == Some("rms/module/v0.1"))
        {
            paths.insert(name.into());
            continue;
        }
        let commits = git(
            root,
            &[
                "log",
                "-m",
                "--format=%H",
                "--diff-filter=D",
                "--no-renames",
                "HEAD",
                "--",
                name,
            ],
        )?;
        'commits: for commit in std::str::from_utf8(&commits)?.lines() {
            let parents = git(root, &["rev-list", "--parents", "-n", "1", commit])?;
            for parent in std::str::from_utf8(&parents)?.split_whitespace().skip(1) {
                if git_yaml_at_revision(root, parent, name)
                    .is_some_and(|v| get_str(&v, &["spec"]) == Some("rms/module/v0.1"))
                {
                    paths.insert(name.into());
                    break 'commits;
                }
            }
        }
    }
    Ok(paths)
}

pub(super) fn validate(root: &Path) -> Result<Vec<Record>> {
    let root = fs::canonicalize(root)?;
    no_symlinks(&root, Path::new(STORE))?;
    if git_head(&root).is_err() {
        if root.join(STORE).exists() {
            bail!("retirement records require Git history");
        }
        return Ok(Vec::new());
    }
    if git(&root, &["rev-parse", "--is-shallow-repository"])? != b"false\n" {
        bail!("module retirement provenance requires complete Git history; fetch the missing history before checking module deletion");
    }
    let mut records = Vec::new();
    let mut retired = BTreeSet::new();
    if root.join(STORE).exists() {
        for entry in fs::read_dir(root.join(STORE))? {
            let entry = entry?;
            let id = entry
                .file_name()
                .to_str()
                .context("invalid retirement id")?
                .to_string();
            if id.len() != 64 || !id.bytes().all(|b| b.is_ascii_hexdigit()) {
                bail!("invalid retirement directory `{id}`");
            }
            no_symlinks(&root, &Path::new(STORE).join(&id))?;
            let directory = entry.path();
            let names = fs::read_dir(&directory)?
                .map(|e| Ok(e?.file_name()))
                .collect::<Result<BTreeSet<_>>>()?;
            if names
                != [
                    std::ffi::OsString::from("archive"),
                    std::ffi::OsString::from("retirement.json"),
                ]
                .into_iter()
                .collect()
            {
                bail!("incomplete or unexpected retirement `{id}`; inspect pending.json and restore the exact archive before retrying");
            }
            no_symlinks(&root, &Path::new(STORE).join(&id).join("retirement.json"))?;
            let record: Record =
                serde_json::from_slice(&fs::read(directory.join("retirement.json"))?)?;
            validate_seal(&record.plan, &record.receipt)?;
            if record.spec != RECORD_SPEC || id != plan_id(&record.plan)? {
                bail!("invalid retirement record identity");
            }
            let original = manifest_directory(&record.plan.manifest)?;
            no_symlinks(&root, original)?;
            if root.join(original).exists() {
                bail!(
                    "retired directory `{}` is active again; implicit restoration is not supported",
                    original.display()
                );
            }
            git(
                &root,
                &["merge-base", "--is-ancestor", &record.plan.base, "HEAD"],
            )?;
            if record.plan.files != git_inventory(&root, &record.plan.base, original)?
                || record.plan.files != filesystem_inventory(&directory.join("archive"))?
            {
                bail!("retirement `{id}` archive differs from its exact committed inventory");
            }
            let value: YamlValue =
                serde_yaml::from_slice(&fs::read(directory.join("archive/module.yaml"))?)?;
            if leaf_policy(&value)? != record.plan.module {
                bail!("retirement module identity mismatch");
            }
            for file in record.plan.files.keys() {
                let bytes = fs::read(directory.join("archive").join(file))?;
                if let Ok(text) = std::str::from_utf8(&bytes) {
                    screen_reference(&original.join(file), text, original, &record.plan.module)?;
                }
            }
            references(&root, original, &record.plan.module)?;
            if !retired.insert(record.plan.manifest.clone()) {
                bail!("duplicate retirement target");
            }
            records.push(record);
        }
    }
    for missing in absent_manifests(&root)?.difference(&retired) {
        bail!("unrecorded RMS module deletion `{missing}`; restore the directory and use an explicit retirement plan");
    }
    records.sort_by(|a, b| a.plan.manifest.cmp(&b.plan.manifest));
    Ok(records)
}

pub(super) fn covers(records: &[Record], path: &Path) -> bool {
    records.iter().any(|r| {
        let directory = Path::new(&r.plan.manifest).parent().unwrap();
        path.strip_prefix(directory)
            .ok()
            .and_then(|p| p.to_str())
            .is_some_and(|p| r.plan.files.contains_key(p))
            || plan_id(&r.plan).is_ok_and(|id| path.starts_with(Path::new(STORE).join(id)))
    })
}

pub(super) fn append_check(root: &Path, report: &mut CheckReport) {
    let (result, summary) = match validate(root) {
        Ok(records) => ("pass", format!("{} retirement archive(s) validated; no unrecorded module deletion; no native behavior certification", records.len())),
        Err(e) => {
            report.result = CheckResult::Fail;
            report.summary = "RMS check fails: module retirement provenance is invalid.".into();
            report.reasons.push(format!("module-retirements: {e:#}"));
            report.next_action = check_follow_up(root, report.mode, report.result);
            ("fail", format!("{e:#}"))
        }
    };
    report.components.push(surface_projection::CheckComponent {
        id: "module-retirements".into(),
        subject: root.display().to_string(),
        scope: RECORD_SPEC.into(),
        result: result.into(),
        summary,
    });
}

pub(super) fn append_audit(root: &Path, checks: &mut Vec<AuditCheck>) {
    let (result, note) = match validate(root) {
        Ok(records) => (
            "pass",
            format!(
                "{} historical retirement archive(s) validated; no unrecorded module deletion",
                records.len()
            ),
        ),
        Err(e) => ("fail", format!("{e:#}")),
    };
    checks.push(audit_check(
        "provenance.module-retirements",
        "provenance",
        result,
        root,
        note,
    ));
}

pub fn run(command: RetirementCommand) -> Result<()> {
    let output = match command {
        RetirementCommand::Plan {
            root,
            module,
            reason,
        } => plan(
            &fs::canonicalize(root)?,
            module.to_str().context("non-UTF-8 manifest")?,
            &reason,
        )?,
        RetirementCommand::Apply {
            root,
            module,
            route_receipt,
            dry_run,
        } => apply(
            &fs::canonicalize(root)?,
            module.to_str().context("non-UTF-8 manifest")?,
            &route_receipt,
            dry_run,
        )?,
        RetirementCommand::Check { root } => {
            json!({"result":"pass", "retirements":validate(&root)?, "scope":"historical provenance only; active and native proof remain separate"})
        }
    };
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    const MANIFEST: &str = "modules/obsolete/module.yaml";

    struct Repo(PathBuf);
    impl Repo {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "rms-retirement-{}-{}",
                std::process::id(),
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            fs::create_dir_all(root.join("modules/obsolete/src")).unwrap();
            let root = fs::canonicalize(root).unwrap();
            fs::write(root.join(MANIFEST), "spec: rms/module/v0.1\nmodule: {name: obsolete, kind: adapter, version: 0.1.0, purpose: Unused scaffold}\nowns: {concepts: [], data: [], decisions: []}\nprovides: {commands: [], queries: [], events: [], capabilities: []}\nrequires: {modules: [], capabilities: []}\neffects: []\ninvariants: []\n").unwrap();
            fs::write(root.join("modules/obsolete/implementation.yaml"), "spec: rms/implementation/v0.2\nmodule: obsolete\nbinding: js\nsource: {root: ., public_entrypoint: src/adapter.mjs}\n").unwrap();
            fs::write(
                root.join("modules/obsolete/src/adapter.mjs"),
                "export const scaffold = true;\n",
            )
            .unwrap();
            fs::write(
                root.join("native.js"),
                "export const website = 'unchanged';\n",
            )
            .unwrap();
            fs::write(
                root.join(".gitignore"),
                ".rms/runs/\n.rms/cache/\nignored/\n",
            )
            .unwrap();
            git(&root, &["init", "-q"]).unwrap();
            git(
                &root,
                &["config", "user.email", "retirement@example.invalid"],
            )
            .unwrap();
            git(&root, &["config", "user.name", "Retirement Test"]).unwrap();
            let repo = Self(root);
            repo.commit();
            repo
        }
        fn commit(&self) {
            git(&self.0, &["add", "-A"]).unwrap();
            git(&self.0, &["commit", "-qm", "fixture", "--allow-empty"]).unwrap();
        }
        fn receipt(&self) -> PathBuf {
            PathBuf::from(
                plan(
                    &self.0,
                    MANIFEST,
                    "Archive unused scaffold; preserve native website",
                )
                .unwrap()["route_receipt"]
                    .as_str()
                    .unwrap(),
            )
        }
        fn retire(&self) -> PathBuf {
            let receipt = self.receipt();
            let value = apply(&self.0, MANIFEST, &receipt, false).unwrap();
            PathBuf::from(value["record"].as_str().unwrap())
                .parent()
                .unwrap()
                .to_path_buf()
        }
    }
    impl Drop for Repo {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn retirement_roundtrip_dry_run_discovery_and_affected_committed_selection() {
        let repo = Repo::new();
        let baseline = filesystem_inventory(&repo.0.join("modules/obsolete")).unwrap();
        let receipt = repo.receipt();
        let status = git(&repo.0, &["status", "--porcelain", "--ignored"]).unwrap();
        apply(&repo.0, MANIFEST, &receipt, true).unwrap();
        assert!(!repo.0.join(STORE).exists());
        assert_eq!(
            status,
            git(&repo.0, &["status", "--porcelain", "--ignored"]).unwrap()
        );
        let output = apply(&repo.0, MANIFEST, &receipt, false).unwrap();
        let record = PathBuf::from(output["record"].as_str().unwrap());
        let schema: JsonValue = serde_json::from_str(include_str!(
            "../../../../schemas/module-retirement.schema.json"
        ))
        .unwrap();
        let validator = jsonschema::validator_for(&schema).unwrap();
        let record_value: JsonValue = serde_json::from_slice(&fs::read(&record).unwrap()).unwrap();
        assert!(
            validator.is_valid(&record_value),
            "{:?}",
            validator.iter_errors(&record_value).collect::<Vec<_>>()
        );
        assert!(validator.is_valid(&record_value["plan"]));
        assert_eq!(
            baseline,
            filesystem_inventory(&record.parent().unwrap().join("archive")).unwrap()
        );
        assert_eq!(
            fs::read_to_string(repo.0.join("native.js")).unwrap(),
            "export const website = 'unchanged';\n"
        );
        assert!(discover_module_index(&repo.0).unwrap().is_empty());
        assert!(discover_implementation_manifests(&repo.0, true)
            .unwrap()
            .is_empty());
        assert_eq!(validate(&repo.0).unwrap().len(), 1);
        for mode in [CheckMode::Changes, CheckMode::Committed] {
            if mode == CheckMode::Committed {
                repo.commit();
            }
            let selection = build_check_selection(&repo.0, mode).unwrap();
            assert!(!selection.paths.is_empty());
            assert!(
                selection
                    .paths
                    .iter()
                    .all(|p| p.coverage == ChangedPathCoverage::RetirementProvenance),
                "{selection:?}"
            );
            assert!(selection.closures.is_empty());
            let report = build_check_report(&repo.0, mode).unwrap();
            assert_eq!(report.result, CheckResult::Pass, "{}", report.summary);
            assert!(report
                .components
                .iter()
                .any(|c| c.id == "module-retirements" && c.result == "pass"));
            assert!(report.delta.outside_coverage_changed_paths.is_empty());
        }
        let mut checks = Vec::new();
        append_audit(&repo.0, &mut checks);
        assert_eq!(checks[0].result, "pass");
    }

    #[test]
    fn retirement_refuses_dirty_or_untracked_or_ignored_inventory() {
        let repo = Repo::new();
        fs::write(repo.0.join("modules/obsolete/src/adapter.mjs"), "changed").unwrap();
        assert!(repo_plan_error(&repo).contains("clean"));
        repo.commit();
        fs::create_dir_all(repo.0.join("modules/obsolete/ignored")).unwrap();
        fs::write(
            repo.0.join("modules/obsolete/ignored/data"),
            "ignored bytes",
        )
        .unwrap();
        assert!(repo_plan_error(&repo).contains("inventory"));
    }

    fn repo_plan_error(repo: &Repo) -> String {
        observe(&repo.0, MANIFEST, "retire")
            .unwrap_err()
            .to_string()
    }

    #[test]
    fn retirement_refuses_owned_meaning_consumers_and_external_source() {
        for (path, content, expected) in [
            (
                "consumer.yaml",
                "requires: {modules: [obsolete]}\n",
                "external reference",
            ),
            (
                "consumer.mjs",
                "import './modules/obsolete/src/adapter.mjs';\n",
                "external reference",
            ),
            (
                "modules/obsolete/implementation.yaml",
                "spec: rms/implementation/v0.2\nsource: {root: ../../native}\n",
                "source.root",
            ),
            (
                "modules/obsolete/src/adapter.mjs",
                "import '../../../native.js';\n",
                "external or ambiguous",
            ),
            (
                "modules/obsolete/nested/module.yaml",
                "spec: rms/module/v0.1\nmodule: {name: child}\n",
                "nested module",
            ),
        ] {
            let repo = Repo::new();
            fs::create_dir_all(repo.0.join(path).parent().unwrap()).unwrap();
            fs::write(repo.0.join(path), content).unwrap();
            repo.commit();
            assert!(
                repo_plan_error(&repo).contains(expected),
                "{}",
                repo_plan_error(&repo)
            );
        }
        for field in ["owns", "provides", "requires", "effects", "composition"] {
            let mut value: YamlValue =
                serde_yaml::from_str("spec: rms/module/v0.1\nmodule: {name: obsolete}\n").unwrap();
            value[field] = serde_yaml::from_str("[meaning]").unwrap();
            assert!(leaf_policy(&value).is_err());
        }
    }

    #[test]
    fn retirement_rejects_stale_wrong_target_and_tampered_plan_receipt() {
        let repo = Repo::new();
        let receipt = repo.receipt();
        assert!(apply(&repo.0, "modules/other/module.yaml", &receipt, true).is_err());
        let path = receipt.parent().unwrap().join("retirement-plan.json");
        let original = fs::read(&path).unwrap();
        let mut value: Plan = serde_json::from_slice(&original).unwrap();
        value.reason = "different decision".into();
        fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
        assert!(apply(&repo.0, MANIFEST, &receipt, true)
            .unwrap_err()
            .to_string()
            .contains("seal"));
        fs::write(&path, original).unwrap();
        let mut value: RouteReceipt = serde_json::from_slice(&fs::read(&receipt).unwrap()).unwrap();
        for result in ["clarification-required", "blocked"] {
            value.payload.route_result = result.into();
            value.receipt_id = sha256_bytes(&serde_json::to_vec(&value.payload).unwrap());
            fs::write(&receipt, serde_json::to_vec(&value).unwrap()).unwrap();
            assert!(apply(&repo.0, MANIFEST, &receipt, true).is_err());
        }
        let receipt = repo.receipt();
        repo.commit();
        assert!(apply(&repo.0, MANIFEST, &receipt, true)
            .unwrap_err()
            .to_string()
            .contains("stale"));
    }

    #[test]
    fn retirement_detects_archive_tampering_and_committed_record_removal() {
        let repo = Repo::new();
        let directory = repo.retire();
        repo.commit();
        let path = directory.join("archive/src/adapter.mjs");
        fs::write(&path, "tampered").unwrap();
        assert!(validate(&repo.0)
            .unwrap_err()
            .to_string()
            .contains("inventory"));
        assert!(build_check_selection(&repo.0, CheckMode::Changes).is_err());
        fs::remove_dir_all(directory).unwrap();
        repo.commit();
        assert!(validate(&repo.0)
            .unwrap_err()
            .to_string()
            .contains("unrecorded"));
        assert!(build_check_selection(&repo.0, CheckMode::Committed).is_err());
    }

    #[test]
    fn retirement_detects_unrecorded_working_and_historical_deletion() {
        let repo = Repo::new();
        fs::remove_dir_all(repo.0.join("modules/obsolete")).unwrap();
        assert!(validate(&repo.0)
            .unwrap_err()
            .to_string()
            .contains("unrecorded"));
        repo.commit();
        assert!(validate(&repo.0)
            .unwrap_err()
            .to_string()
            .contains("unrecorded"));
    }

    #[test]
    fn retirement_history_survives_clone_but_shallow_history_cannot_certify_deletion() {
        let repo = Repo::new();
        repo.retire();
        repo.commit();
        let clone = repo.0.join("clone");
        git(
            &repo.0,
            &[
                "clone",
                "--quiet",
                "--no-hardlinks",
                repo.0.to_str().unwrap(),
                clone.to_str().unwrap(),
            ],
        )
        .unwrap();
        assert_eq!(validate(&clone).unwrap().len(), 1);
        let repo = Repo::new();
        let shallow = repo.0.join("shallow");
        git(
            &repo.0,
            &[
                "clone",
                "--quiet",
                "--depth=1",
                &format!("file://{}", repo.0.display()),
                shallow.to_str().unwrap(),
            ],
        )
        .unwrap();
        assert!(validate(&shallow)
            .unwrap_err()
            .to_string()
            .contains("complete Git history"));
    }

    #[test]
    fn retirement_rejects_unsafe_paths_and_partial_archive() {
        for path in [
            "module.yaml",
            "../module.yaml",
            "/tmp/module.yaml",
            ".rms/foo/module.yaml",
            "modules/../foo/module.yaml",
        ] {
            assert!(manifest_directory(path).is_err());
        }
        let repo = Repo::new();
        let directory = repo.retire();
        fs::rename(
            directory.join("retirement.json"),
            directory.join("pending.json"),
        )
        .unwrap();
        assert!(validate(&repo.0)
            .unwrap_err()
            .to_string()
            .contains("incomplete"));
    }

    #[cfg(unix)]
    #[test]
    fn retirement_preserves_executable_mode_and_refuses_links() {
        use std::os::unix::fs::{symlink, PermissionsExt};
        let repo = Repo::new();
        let source = repo.0.join("modules/obsolete/src/adapter.mjs");
        fs::set_permissions(&source, fs::Permissions::from_mode(0o755)).unwrap();
        repo.commit();
        let directory = repo.retire();
        assert!(
            filesystem_inventory(&directory.join("archive")).unwrap()["src/adapter.mjs"].executable
        );
        fs::set_permissions(
            directory.join("archive/src/adapter.mjs"),
            fs::Permissions::from_mode(0o644),
        )
        .unwrap();
        assert!(validate(&repo.0).is_err());
        let repo = Repo::new();
        symlink(
            "../../../native.js",
            repo.0.join("modules/obsolete/src/link"),
        )
        .unwrap();
        repo.commit();
        assert!(repo_plan_error(&repo).contains("non-regular"));
        let repo = Repo::new();
        fs::hard_link(
            repo.0.join("native.js"),
            repo.0.join("modules/obsolete/src/hardlink"),
        )
        .unwrap();
        repo.commit();
        assert!(repo_plan_error(&repo).contains("hard-linked"));
    }
}
