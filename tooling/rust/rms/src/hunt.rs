use super::{execute_proof_command, get_path, get_str, sha256_bytes};
use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use serde_yaml::Value as YamlValue;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering as AtomicOrdering};
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use walkdir::WalkDir;

const REPORT_SPEC: &str = "rms/hunt-report/v0.2";
const LANE_RESULT_SPEC: &str = "rms/hunt-lane-result/v0.1";
const MAX_GUIDED_FINDINGS: usize = 8;

pub(super) struct HuntRequest {
    root: PathBuf,
    module: Option<PathBuf>,
    assembly: Option<PathBuf>,
    budget: Option<String>,
    seed: Option<u64>,
    jobs: Option<usize>,
    resume: Option<String>,
    out: Option<PathBuf>,
    dry_run: bool,
    json: bool,
}

impl HuntRequest {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn from_cli(
        root: PathBuf,
        module: Option<PathBuf>,
        assembly: Option<PathBuf>,
        budget: Option<String>,
        seed: Option<u64>,
        jobs: Option<usize>,
        resume: Option<String>,
        out: Option<PathBuf>,
        dry_run: bool,
        json: bool,
    ) -> Self {
        Self {
            root,
            module,
            assembly,
            budget,
            seed,
            jobs,
            resume,
            out,
            dry_run,
            json,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct HuntSource {
    revision: String,
    declaration_digest: String,
    root: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct HuntConfiguration {
    budget_seconds: u64,
    seed: u64,
    jobs: usize,
    module: Option<String>,
    #[serde(default)]
    assembly: Option<String>,
    #[serde(default)]
    output: Option<String>,
    tools: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct HuntLaneReport {
    id: String,
    module: String,
    property: Option<String>,
    kind: String,
    strategy: String,
    status: String,
    budget_seconds: u64,
    elapsed_ms: u64,
    command: String,
    runner: Option<String>,
    #[serde(default)]
    metrics: BTreeMap<String, JsonValue>,
    #[serde(default)]
    artifacts: Vec<String>,
    diagnostic: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct HuntFinding {
    #[serde(default)]
    id: String,
    lane: String,
    #[serde(default)]
    module: String,
    #[serde(default)]
    property: Option<String>,
    kind: String,
    summary: String,
    #[serde(default)]
    check: Option<String>,
    #[serde(default)]
    first_bad_transition: Option<String>,
    #[serde(default = "one_occurrence")]
    occurrences: usize,
    artifact: Option<String>,
    replay: Option<String>,
}

fn one_occurrence() -> usize {
    1
}

impl HuntFinding {
    fn raw(
        lane: String,
        kind: String,
        summary: String,
        artifact: Option<String>,
        replay: Option<String>,
    ) -> Self {
        Self {
            id: String::new(),
            lane,
            module: String::new(),
            property: None,
            kind,
            summary,
            check: None,
            first_bad_transition: None,
            occurrences: 1,
            artifact,
            replay,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct HuntProofScope {
    claim: String,
    exhausted_lanes: Vec<String>,
    bounded_lanes: Vec<String>,
    unsupported_lanes: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct HuntReport {
    spec: String,
    run_id: String,
    result: String,
    source: HuntSource,
    configuration: HuntConfiguration,
    lanes: Vec<HuntLaneReport>,
    findings: Vec<HuntFinding>,
    proof_scope: HuntProofScope,
    #[serde(default)]
    elapsed_ms: u64,
    started_at_unix_ms: u64,
    finished_at_unix_ms: Option<u64>,
}

#[derive(Clone, Debug)]
enum LaneExecution {
    Rms {
        root: PathBuf,
        arguments: Vec<String>,
    },
    Project {
        root: PathBuf,
        command: String,
        property: String,
        runner: String,
        generator: Option<String>,
    },
    GuidedProbe {
        root: PathBuf,
        source_root: PathBuf,
        assembly: PathBuf,
        max_steps: Option<usize>,
        max_schedules: Option<usize>,
        max_states: Option<usize>,
    },
}

#[derive(Clone, Debug)]
struct HuntLane {
    report: HuntLaneReport,
    execution: LaneExecution,
}

#[derive(Clone, Debug, Deserialize)]
struct DeclaredRealization {
    profile: String,
    strategy: String,
    command: String,
    runner: String,
    #[serde(default)]
    generator: Option<String>,
    #[serde(default)]
    exhaustive: bool,
}

pub(super) fn parse_budget(value: &str) -> std::result::Result<String, String> {
    parse_duration_seconds(value)
        .map(|_| value.to_string())
        .map_err(|error| error.to_string())
}

pub(super) fn run(request: HuntRequest) -> Result<()> {
    let root = request.root.canonicalize().with_context(|| {
        format!(
            "hunt root `{}` could not be resolved",
            request.root.display()
        )
    })?;
    let hunts_root = root.join(".rms/hunts");
    fs::create_dir_all(&hunts_root)?;
    let resumed = load_resume_report(&hunts_root, request.resume.as_deref())?;
    let prior_report = resumed.as_ref().map(|(_, report)| report);

    let requested_module = request
        .module
        .as_deref()
        .map(|path| resolve_hunt_file(&root, path, "module"))
        .transpose()?;
    let relative_module = match (requested_module, prior_report) {
        (Some(module), Some(previous)) => {
            let recorded = previous.configuration.module.as_deref().map(Path::new);
            if recorded != Some(module.as_path()) {
                bail!("hunt resume rejected module configuration drift");
            }
            Some(module)
        }
        (Some(module), None) => Some(module),
        (None, Some(previous)) => previous.configuration.module.as_deref().map(PathBuf::from),
        (None, None) => None,
    };
    let requested_assembly = request
        .assembly
        .as_deref()
        .map(|path| resolve_hunt_file(&root, path, "assembly"))
        .transpose()?;
    let relative_assembly = match (requested_assembly, prior_report) {
        (Some(assembly), Some(previous)) => {
            let recorded = previous.configuration.assembly.as_deref().map(Path::new);
            if recorded != Some(assembly.as_path()) {
                bail!("hunt resume rejected assembly configuration drift");
            }
            Some(assembly)
        }
        (Some(assembly), None) => Some(assembly),
        (None, Some(previous)) => previous
            .configuration
            .assembly
            .as_deref()
            .map(PathBuf::from),
        (None, None) => None,
    };
    if relative_module.is_some() && relative_assembly.is_some() {
        bail!("hunt configuration cannot select both a module and an assembly");
    }

    let requested_budget = request
        .budget
        .as_deref()
        .map(parse_duration_seconds)
        .transpose()?;
    let budget_seconds = match (requested_budget, prior_report) {
        (Some(budget), Some(previous)) if budget != previous.configuration.budget_seconds => {
            bail!("hunt resume rejected budget configuration drift")
        }
        (Some(budget), _) => budget,
        (None, Some(previous)) => previous.configuration.budget_seconds,
        (None, None) => parse_duration_seconds("8h")?,
    };

    if request.jobs == Some(0) {
        bail!("hunt jobs must be at least 1");
    }
    let configured_jobs = match (request.jobs, prior_report) {
        (Some(jobs), Some(previous)) if jobs != previous.configuration.jobs => {
            bail!("hunt resume rejected jobs configuration drift")
        }
        (Some(jobs), _) => jobs,
        (None, Some(previous)) => previous.configuration.jobs,
        (None, None) => 4,
    };
    let jobs = configured_jobs.min(
        std::thread::available_parallelism()
            .map(usize::from)
            .unwrap_or(1),
    );

    let requested_output = request
        .out
        .as_deref()
        .map(|path| absolute_output_path(&root, path));
    let output = match (requested_output, prior_report) {
        (Some(path), Some(previous))
            if previous.configuration.output.as_deref()
                != Some(path.to_string_lossy().as_ref()) =>
        {
            bail!("hunt resume rejected output configuration drift")
        }
        (Some(path), _) => Some(path),
        (None, Some(previous)) => previous.configuration.output.as_deref().map(PathBuf::from),
        (None, None) => None,
    };

    let revision = git_output(&root, &["rev-parse", "HEAD"])?;
    ensure_clean_commit(&root)?;
    let declaration_digest = declaration_digest(
        &root,
        relative_module.as_deref(),
        relative_assembly.as_deref(),
    )?;
    let (run_id, seed, mut prior) = if let Some((run_id, report)) = resumed {
        validate_resume(&report, request.seed, &revision, &declaration_digest)?;
        (run_id, report.configuration.seed, Some(report))
    } else {
        let seed = request.seed.unwrap_or_else(generate_seed);
        (new_run_id(&revision), seed, None)
    };
    if let Some(final_report) = prior.as_ref().filter(|report| {
        report.finished_at_unix_ms.is_some()
            && report.lanes.iter().all(|lane| {
                matches!(
                    lane.status.as_str(),
                    "pass" | "finding" | "invalid" | "unsupported"
                )
            })
    }) {
        return print_report(final_report, request.json);
    }
    let run_root = hunts_root.join(&run_id);
    prepare_run_storage(&run_root)?;
    let prior_elapsed_ms = prior.as_ref().map(resume_elapsed_ms).unwrap_or_default();
    let started_at_unix_ms = prior
        .as_ref()
        .map(|report| report.started_at_unix_ms)
        .unwrap_or_else(now_ms);
    let worktree = run_root.join("checkout");
    prepare_isolated_checkout(&root, &worktree, &revision)?;
    let mut lanes = if let Some(assembly) = relative_assembly.as_deref() {
        vec![discover_direct_assembly_lane(
            &worktree,
            &root,
            assembly,
            budget_seconds,
        )?]
    } else {
        discover_lanes(
            &worktree,
            &root,
            relative_module.as_deref(),
            budget_seconds,
            &run_root,
        )?
    };
    let tools = tool_identities(&mut lanes);
    if let Some(previous) = &prior {
        ensure_resume_tool_identities(previous, &tools)?;
    }
    if let Some(previous) = prior.as_mut() {
        normalize_interrupted_resume(previous, &run_root)?;
    }
    let previous_lanes = prior
        .as_ref()
        .map(|report| {
            report
                .lanes
                .iter()
                .map(|lane| (lane.id.clone(), lane.clone()))
                .collect::<BTreeMap<_, _>>()
        })
        .unwrap_or_default();
    let completed = prior
        .as_ref()
        .map(|report| {
            report
                .lanes
                .iter()
                .filter(|lane| {
                    matches!(
                        lane.status.as_str(),
                        "pass" | "finding" | "invalid" | "unsupported"
                    )
                })
                .map(|lane| (lane.id.clone(), lane.clone()))
                .collect::<BTreeMap<_, _>>()
        })
        .unwrap_or_default();
    for lane in &mut lanes {
        if let Some(previous) = completed.get(&lane.report.id) {
            lane.report = previous.clone();
        } else if let Some(previous) = previous_lanes.get(&lane.report.id) {
            lane.report.elapsed_ms = previous.elapsed_ms;
            lane.report.diagnostic = previous.diagnostic.clone();
        }
    }
    let lane_count = lanes
        .iter()
        .filter(|lane| !completed.contains_key(&lane.report.id) && lane.report.status == "planned")
        .count()
        .max(1);
    let parallelism = jobs.min(lane_count);
    let per_lane_budget = budget_seconds
        .saturating_mul(parallelism as u64)
        .checked_div(lane_count as u64)
        .unwrap_or(1)
        .max(1);
    for lane in &mut lanes {
        lane.report.budget_seconds = lane.report.budget_seconds.min(per_lane_budget).max(1);
    }
    let mut report = HuntReport {
        spec: REPORT_SPEC.to_string(),
        run_id: run_id.clone(),
        result: "inconclusive".to_string(),
        source: HuntSource {
            revision: revision.clone(),
            declaration_digest,
            root: root.display().to_string(),
        },
        configuration: HuntConfiguration {
            budget_seconds,
            seed,
            jobs: configured_jobs,
            module: relative_module
                .as_ref()
                .map(|path| path.display().to_string()),
            assembly: relative_assembly
                .as_ref()
                .map(|path| path.display().to_string()),
            output: output.as_ref().map(|path| path.display().to_string()),
            tools,
        },
        lanes: lanes.iter().map(|lane| lane.report.clone()).collect(),
        findings: prior
            .take()
            .map(|report| report.findings)
            .unwrap_or_default(),
        proof_scope: empty_scope(),
        elapsed_ms: prior_elapsed_ms,
        started_at_unix_ms,
        finished_at_unix_ms: None,
    };
    write_report(&run_root.join("checkpoint.yaml"), &report)?;
    if request.dry_run {
        normalize_report_findings(&mut report);
        report.result = if lanes.is_empty() {
            "unsupported".to_string()
        } else {
            "inconclusive".to_string()
        };
        report.proof_scope = proof_scope(&report.lanes);
        report.finished_at_unix_ms = Some(now_ms());
        finish_report(&root, &worktree, &run_root, output.as_deref(), &report)?;
        return print_report(&report, request.json);
    }

    let hunt_started = Instant::now();
    let base_elapsed_ms = report.elapsed_ms;
    let budget_ms = budget_seconds.saturating_mul(1_000);
    let hunt_deadline = hunt_started
        .checked_add(Duration::from_millis(
            budget_ms.saturating_sub(base_elapsed_ms),
        ))
        .unwrap_or(hunt_started);
    let cancellation = Arc::new(AtomicBool::new(false));
    let pending = lanes
        .iter()
        .enumerate()
        .filter(|(_, lane)| {
            !completed.contains_key(&lane.report.id) && lane.report.status == "planned"
        })
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    let mut cursor = 0;
    while cursor < pending.len() {
        report.elapsed_ms =
            base_elapsed_ms.saturating_add(hunt_started.elapsed().as_millis() as u64);
        let Some(remaining) = remaining_timeout_seconds(hunt_deadline) else {
            cancellation.store(true, AtomicOrdering::Release);
            for index in &pending[cursor..] {
                lanes[*index].report.status = "inconclusive".to_string();
                lanes[*index].report.diagnostic = Some("total hunt budget exhausted".to_string());
            }
            break;
        };
        let phase = lane_phase(&lanes[pending[cursor]]);
        let phase_end = pending[cursor..]
            .iter()
            .position(|index| lane_phase(&lanes[*index]) != phase)
            .map(|offset| cursor + offset)
            .unwrap_or(pending.len());
        let phase_parallelism = phase_parallelism(phase, jobs);
        let batch_end = (cursor + phase_parallelism)
            .min(phase_end)
            .min(pending.len());
        let batch = &pending[cursor..batch_end];
        if !request.json {
            let lane_names = batch
                .iter()
                .map(|index| lanes[*index].report.id.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            eprintln!("hunt: running {lane_names} (up to {remaining}s remaining)");
        }
        for index in batch {
            lanes[*index].report.status = "running".to_string();
            lanes[*index].report.budget_seconds =
                lanes[*index].report.budget_seconds.min(remaining);
        }
        report.lanes = lanes.iter().map(|lane| lane.report.clone()).collect();
        write_report(&run_root.join("checkpoint.yaml"), &report)?;

        std::thread::scope(|scope| -> Result<()> {
            let (sender, receiver) = mpsc::channel();
            for index in batch.iter().copied() {
                let lane = lanes[index].clone();
                let sender = sender.clone();
                let run_id = run_id.clone();
                let run_root = run_root.clone();
                let cancellation = Arc::clone(&cancellation);
                scope.spawn(move || {
                    let result = execute_lane(
                        &lane,
                        &run_id,
                        seed.wrapping_add(index as u64),
                        &run_root,
                        hunt_deadline,
                        cancellation,
                    );
                    let _ = sender.send((index, result));
                });
            }
            drop(sender);

            let mut received = 0usize;
            let mut last_checkpoint = Instant::now();
            while received < batch.len() {
                if Instant::now() >= hunt_deadline {
                    cancellation.store(true, AtomicOrdering::Release);
                }
                match receiver.recv_timeout(Duration::from_millis(100)) {
                    Ok((index, lane_result)) => {
                        received += 1;
                        match lane_result {
                            Ok((lane_report, findings)) => {
                                lanes[index].report = lane_report;
                                report.findings.extend(findings);
                            }
                            Err(error) => {
                                let deadline_exhausted = Instant::now() >= hunt_deadline
                                    || cancellation.load(AtomicOrdering::Acquire);
                                lanes[index].report.status = if deadline_exhausted {
                                    "inconclusive"
                                } else {
                                    "invalid"
                                }
                                .to_string();
                                lanes[index].report.diagnostic = Some(error.to_string());
                            }
                        }
                        report.elapsed_ms = base_elapsed_ms
                            .saturating_add(hunt_started.elapsed().as_millis() as u64);
                        report.lanes = lanes.iter().map(|lane| lane.report.clone()).collect();
                        write_report(&run_root.join("checkpoint.yaml"), &report)?;
                        last_checkpoint = Instant::now();
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        if last_checkpoint.elapsed() >= Duration::from_secs(1) {
                            report.elapsed_ms = base_elapsed_ms
                                .saturating_add(hunt_started.elapsed().as_millis() as u64);
                            report.lanes = lanes.iter().map(|lane| lane.report.clone()).collect();
                            write_report(&run_root.join("checkpoint.yaml"), &report)?;
                            last_checkpoint = Instant::now();
                        }
                    }
                    Err(mpsc::RecvTimeoutError::Disconnected) => {
                        bail!("hunt lane result channel disconnected")
                    }
                }
            }
            Ok(())
        })?;
        cursor = batch_end;
    }
    report.elapsed_ms = base_elapsed_ms.saturating_add(hunt_started.elapsed().as_millis() as u64);
    report.lanes = lanes.into_iter().map(|lane| lane.report).collect();
    normalize_report_findings(&mut report);
    report.result = hunt_result(&report);
    report.proof_scope = proof_scope(&report.lanes);
    report.finished_at_unix_ms = Some(now_ms());
    finish_report(&root, &worktree, &run_root, output.as_deref(), &report)?;
    print_report(&report, request.json)?;
    if matches!(report.result.as_str(), "invalid" | "unsupported") {
        bail!("RMS hunt {}", report.result);
    }
    Ok(())
}

fn prepare_run_storage(run_root: &Path) -> Result<()> {
    fs::create_dir_all(run_root.join("analyses")).with_context(|| {
        format!(
            "failed to prepare hunt run storage `{}`",
            run_root.display()
        )
    })
}

fn resume_elapsed_ms(report: &HuntReport) -> u64 {
    if report.elapsed_ms > 0 {
        report.elapsed_ms
    } else {
        report
            .lanes
            .iter()
            .map(|lane| lane.elapsed_ms)
            .max()
            .unwrap_or_default()
    }
}

fn normalize_interrupted_resume(report: &mut HuntReport, run_root: &Path) -> Result<()> {
    let interrupted = report
        .lanes
        .iter()
        .filter(|lane| lane.status == "running")
        .map(|lane| lane.id.clone())
        .collect::<Vec<_>>();
    if interrupted.is_empty() {
        return Ok(());
    }

    let archive_root = run_root.join("interrupted").join(now_ms().to_string());
    for lane_id in &interrupted {
        let segment = sanitize_segment(lane_id);
        let lane_output = run_root.join("lanes").join(format!("{segment}.yaml"));
        let counterexamples = run_root.join("counterexamples").join(&segment);
        for source in [&lane_output, &counterexamples] {
            if !source.exists() {
                continue;
            }
            fs::create_dir_all(&archive_root)?;
            let name = source
                .file_name()
                .ok_or_else(|| anyhow!("interrupted hunt artifact has no file name"))?;
            let destination = archive_root.join(name);
            fs::rename(source, &destination).with_context(|| {
                format!(
                    "failed to preserve interrupted hunt artifact `{}` as `{}`",
                    source.display(),
                    destination.display()
                )
            })?;
        }
    }
    report
        .findings
        .retain(|finding| !interrupted.iter().any(|lane| lane == &finding.lane));
    for lane in &mut report.lanes {
        if interrupted.iter().any(|id| id == &lane.id) {
            lane.status = "planned".to_string();
            lane.diagnostic = Some(format!(
                "previous execution was interrupted; preserved partial artifacts under {} and scheduled an explicit rerun",
                archive_root.display()
            ));
        }
    }
    report.finished_at_unix_ms = None;
    Ok(())
}

fn discover_direct_assembly_lane(
    checkout: &Path,
    source_root: &Path,
    assembly: &Path,
    budget_seconds: u64,
) -> Result<HuntLane> {
    let path = checkout.join(assembly);
    let value = load_yaml_or_json(&path)
        .with_context(|| format!("hunt assembly `{}` could not be loaded", assembly.display()))?;
    if !matches!(
        get_str(&value, &["spec"]),
        Some("rms/probe-assembly/v0.1" | "rms/probe-assembly/v0.2")
    ) {
        bail!(
            "hunt assembly `{}` is not an rms/probe-assembly/v0.1 or v0.2 declaration",
            assembly.display()
        );
    }
    let max_steps = get_path(&value, &["exploration", "max_steps"])
        .and_then(YamlValue::as_u64)
        .map(|value| value as usize);
    let max_schedules = get_path(&value, &["exploration", "max_schedules"])
        .and_then(YamlValue::as_u64)
        .map(|value| value as usize);
    let max_states = get_path(&value, &["exploration", "max_states"])
        .and_then(YamlValue::as_u64)
        .map(|value| value as usize);
    let name = assembly
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("probe-assembly");
    let coverage_gaps = super::probe::campaign_planning_gaps(&path, checkout)?;
    let mut lane = HuntLane {
        report: lane_report(
            format!("assembly:{}:guided", sanitize_segment(name)),
            &format!("assembly:{name}"),
            None,
            "probe-exploration",
            "guided-semantic-novelty-v1",
            budget_seconds,
            format!(
                "guided probe exploration {} (seeded, at most {MAX_GUIDED_FINDINGS} distinct findings)",
                assembly.display()
            ),
            None,
        ),
        execution: LaneExecution::GuidedProbe {
            root: checkout.to_path_buf(),
            source_root: source_root.to_path_buf(),
            assembly: path,
            max_steps,
            max_schedules,
            max_states,
        },
    };
    if !coverage_gaps.is_empty() {
        lane.report.status = "unsupported".to_string();
        lane.report.diagnostic = Some(format!(
            "composition hunt campaign coverage is incomplete: {}",
            coverage_gaps.join("; ")
        ));
    }
    Ok(lane)
}

fn discover_lanes(
    checkout: &Path,
    source_root: &Path,
    selected_module: Option<&Path>,
    budget_seconds: u64,
    run_root: &Path,
) -> Result<Vec<HuntLane>> {
    let mut implementations = WalkDir::new(checkout)
        .max_depth(12)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| {
            !matches!(
                entry.file_name().to_str(),
                Some(".git") | Some("target") | Some("node_modules") | Some(".rms")
            )
        })
        .filter_map(std::result::Result::ok)
        .filter(|entry| entry.file_type().is_file() && entry.file_name() == "implementation.yaml")
        .map(|entry| entry.into_path())
        .collect::<Vec<_>>();
    implementations.sort();
    if let Some(module) = selected_module {
        let closure = selected_module_closure(checkout, module)?;
        implementations.retain(|path| {
            load_yaml_or_json(path)
                .ok()
                .and_then(|value| get_str(&value, &["module"]).map(ToString::to_string))
                .is_some_and(|name| closure.contains(&name))
        });
    }
    let mut lanes = Vec::new();
    for implementation in implementations {
        let source = fs::read_to_string(&implementation)?;
        let value: YamlValue = serde_yaml::from_str(&source)?;
        let module = get_str(&value, &["module"])
            .unwrap_or("unknown")
            .to_string();
        let relative_implementation = implementation
            .strip_prefix(checkout)
            .unwrap_or(&implementation)
            .to_path_buf();
        let properties = get_path(&value, &["architecture", "reliability", "properties"])
            .and_then(YamlValue::as_sequence)
            .into_iter()
            .flatten()
            .chain(
                get_path(&value, &["architecture", "reliability", "fuzz_targets"])
                    .and_then(YamlValue::as_sequence)
                    .into_iter()
                    .flatten(),
            )
            .collect::<Vec<_>>();
        let temporal_properties = properties
            .iter()
            .filter(|property| property_has_executable_expression(property))
            .filter_map(|property| get_str(property, &["id"]))
            .collect::<Vec<_>>();
        let trace_producers = get_path(&value, &["architecture", "trace", "producers"])
            .and_then(YamlValue::as_sequence)
            .into_iter()
            .flatten()
            .filter(|producer| get_str(producer, &["profile"]) == Some("smoke"))
            .filter_map(|producer| get_str(producer, &["bundle"]))
            .collect::<Vec<_>>();
        let has_smoke = properties.iter().any(|property| {
            get_path(property, &["realizations"])
                .and_then(YamlValue::as_sequence)
                .is_some_and(|items| {
                    items
                        .iter()
                        .any(|item| get_str(item, &["profile"]) == Some("smoke"))
                })
        });
        if has_smoke {
            let arguments = vec![
                "property".to_string(),
                "run".to_string(),
                relative_implementation.display().to_string(),
                "--profile".to_string(),
                "smoke".to_string(),
                "--timeout-seconds".to_string(),
                budget_seconds.min(300).to_string(),
            ];
            lanes.push(HuntLane {
                report: lane_report(
                    format!("{module}:smoke-baseline"),
                    &module,
                    None,
                    "baseline",
                    "deterministic-baseline",
                    budget_seconds.min(300),
                    rms_command(&arguments),
                    None,
                ),
                execution: LaneExecution::Rms {
                    root: checkout.to_path_buf(),
                    arguments,
                },
            });
        }
        if !temporal_properties.is_empty() && !trace_producers.is_empty() {
            let arguments = vec![
                "trace".to_string(),
                "run".to_string(),
                relative_implementation.display().to_string(),
                "--profile".to_string(),
                "smoke".to_string(),
                "--record".to_string(),
                "--timeout-seconds".to_string(),
                budget_seconds.min(300).to_string(),
            ];
            lanes.push(HuntLane {
                report: lane_report(
                    format!("{module}:trace-regeneration"),
                    &module,
                    None,
                    "trace-regeneration",
                    "real-trace",
                    budget_seconds.min(300),
                    rms_command(&arguments),
                    None,
                ),
                execution: LaneExecution::Rms {
                    root: checkout.to_path_buf(),
                    arguments,
                },
            });
            for property_id in &temporal_properties {
                for (trace_index, bundle) in trace_producers.iter().enumerate() {
                    let analysis = run_root.join("analyses").join(format!(
                        "{}-{}-trace-{}.yaml",
                        sanitize_segment(&module),
                        sanitize_segment(property_id),
                        trace_index + 1
                    ));
                    let arguments = vec![
                        "property".to_string(),
                        "evaluate".to_string(),
                        relative_implementation.display().to_string(),
                        "--trace".to_string(),
                        implementation
                            .parent()
                            .unwrap_or(checkout)
                            .join(bundle)
                            .strip_prefix(checkout)
                            .unwrap_or_else(|_| Path::new(bundle))
                            .display()
                            .to_string(),
                        "--property".to_string(),
                        (*property_id).to_string(),
                        "--out".to_string(),
                        analysis.display().to_string(),
                    ];
                    lanes.push(HuntLane {
                        report: lane_report(
                            format!(
                                "{module}:{property_id}:trace-evaluation-{}",
                                trace_index + 1
                            ),
                            &module,
                            Some((*property_id).to_string()),
                            "trace-evaluation",
                            "real-trace",
                            budget_seconds,
                            rms_command(&arguments),
                            None,
                        ),
                        execution: LaneExecution::Rms {
                            root: checkout.to_path_buf(),
                            arguments,
                        },
                    });
                }
            }
        }
        let mut historical_paths = BTreeMap::<PathBuf, String>::new();
        let mut relationship_assemblies =
            BTreeMap::<PathBuf, (YamlValue, std::collections::BTreeSet<String>, bool)>::new();
        for property in &properties {
            let property_id = get_str(property, &["id"]).unwrap_or("unnamed").to_string();
            let has_executable_expression = property_has_executable_expression(property);
            if let Some(counterexamples) = get_str(property, &["counterexamples", "path"]) {
                let directory = implementation
                    .parent()
                    .unwrap_or(checkout)
                    .join(counterexamples);
                if directory.is_dir() {
                    for entry in WalkDir::new(&directory)
                        .max_depth(4)
                        .follow_links(false)
                        .into_iter()
                        .filter_map(std::result::Result::ok)
                        .filter(|entry| entry.file_type().is_file())
                    {
                        historical_paths
                            .entry(entry.into_path())
                            .or_insert_with(|| property_id.clone());
                    }
                }
            }
            for realization in get_path(property, &["realizations"])
                .and_then(YamlValue::as_sequence)
                .into_iter()
                .flatten()
            {
                let Ok(realization) =
                    serde_yaml::from_value::<DeclaredRealization>(realization.clone())
                else {
                    continue;
                };
                if realization.profile != "nightly" {
                    continue;
                }
                let Some(command) = get_str(&value, &["commands", &realization.command]) else {
                    continue;
                };
                let lane_id = format!(
                    "{module}:{property_id}:{}",
                    sanitize_segment(&realization.strategy)
                );
                let output = run_root
                    .join("lanes")
                    .join(format!("{}.yaml", sanitize_segment(&lane_id)));
                lanes.push(HuntLane {
                    report: lane_report(
                        lane_id,
                        &module,
                        Some(property_id.clone()),
                        "declared-realization",
                        &realization.strategy,
                        budget_seconds,
                        command.to_string(),
                        Some(realization.runner.clone()),
                    ),
                    execution: LaneExecution::Project {
                        root: implementation.parent().unwrap_or(checkout).to_path_buf(),
                        command: command.to_string(),
                        property: property_id.clone(),
                        runner: realization.runner,
                        generator: realization.generator,
                    },
                });
                if realization.strategy == "deterministic-exhaustive" && !realization.exhaustive {
                    if let Some(last) = lanes.last_mut() {
                        last.report.status = "invalid".to_string();
                        last.report.diagnostic =
                            Some("deterministic-exhaustive requires exhaustive: true".to_string());
                    }
                }
                if let Some(parent) = output.parent() {
                    fs::create_dir_all(parent)?;
                }
            }
            for (exploration_index, exploration) in get_path(property, &["explorations"])
                .and_then(YamlValue::as_sequence)
                .into_iter()
                .flatten()
                .enumerate()
            {
                let Some(assembly) = get_str(exploration, &["assembly"]) else {
                    continue;
                };
                relationship_assemblies
                    .entry(implementation.parent().unwrap_or(checkout).join(assembly))
                    .and_modify(|(_, properties, has_executable)| {
                        properties.insert(property_id.clone());
                        *has_executable |= has_executable_expression;
                    })
                    .or_insert_with(|| {
                        (
                            exploration.clone(),
                            std::collections::BTreeSet::from([property_id.clone()]),
                            has_executable_expression,
                        )
                    });
                if !has_executable_expression {
                    continue;
                }
                let goal = get_str(exploration, &["goal"]).unwrap_or("violate");
                let mut arguments = vec![
                    "property".to_string(),
                    "search".to_string(),
                    relative_implementation.display().to_string(),
                    "--assembly".to_string(),
                    implementation
                        .parent()
                        .unwrap_or(checkout)
                        .join(assembly)
                        .strip_prefix(checkout)
                        .unwrap_or_else(|_| Path::new(assembly))
                        .display()
                        .to_string(),
                    "--goal".to_string(),
                    goal.to_string(),
                    "--property".to_string(),
                    property_id.clone(),
                ];
                for (flag, field) in [
                    ("--max-steps", "max_steps"),
                    ("--max-schedules", "max_schedules"),
                    ("--max-states", "max_states"),
                ] {
                    if let Some(value) =
                        get_path(exploration, &["bounds", field]).and_then(YamlValue::as_u64)
                    {
                        arguments.push(flag.to_string());
                        arguments.push(value.to_string());
                    }
                }
                let analysis = run_root.join("analyses").join(format!(
                    "{}-{}-{}.yaml",
                    sanitize_segment(&module),
                    sanitize_segment(&property_id),
                    exploration_index + 1
                ));
                arguments.push("--out".to_string());
                arguments.push(analysis.display().to_string());
                arguments.push("--timeout-seconds".to_string());
                arguments.push(budget_seconds.to_string());
                lanes.push(HuntLane {
                    report: lane_report(
                        format!(
                            "{module}:{property_id}:exploration-{}",
                            exploration_index + 1
                        ),
                        &module,
                        Some(property_id.clone()),
                        "probe-exploration",
                        "probe-search",
                        budget_seconds,
                        rms_command(&arguments),
                        None,
                    ),
                    execution: LaneExecution::Rms {
                        root: checkout.to_path_buf(),
                        arguments,
                    },
                });
            }
        }
        for (path, property_id) in historical_paths {
            let Ok(counterexample) = load_yaml_or_json(&path) else {
                continue;
            };
            let relative_path = path
                .strip_prefix(checkout)
                .unwrap_or(&path)
                .display()
                .to_string();
            let (arguments, strategy) = match get_str(&counterexample, &["spec"]) {
                Some("rms/property-analysis/v0.1" | "rms/property-analysis/v0.2") => (
                    vec![
                        "property".to_string(),
                        "replay".to_string(),
                        relative_path,
                        "--json".to_string(),
                    ],
                    "property-replay",
                ),
                Some("rms/probe-counterexample/v0.1") => (
                    vec![
                        "probe".to_string(),
                        relative_implementation.display().to_string(),
                        "--replay".to_string(),
                        relative_path,
                    ],
                    "probe-replay",
                ),
                _ => continue,
            };
            lanes.push(HuntLane {
                report: lane_report(
                    format!(
                        "{module}:{property_id}:historical-{}",
                        sanitize_segment(
                            path.file_stem()
                                .and_then(|stem| stem.to_str())
                                .unwrap_or("counterexample")
                        )
                    ),
                    &module,
                    Some(property_id),
                    "historical-replay",
                    strategy,
                    budget_seconds.min(120),
                    rms_command(&arguments),
                    None,
                ),
                execution: LaneExecution::Rms {
                    root: checkout.to_path_buf(),
                    arguments,
                },
            });
        }
        for (analysis_index, (assembly, (exploration, properties, has_executable_expression))) in
            relationship_assemblies.into_iter().enumerate()
        {
            let guided_property = (properties.len() == 1)
                .then(|| properties.iter().next().cloned())
                .flatten();
            let max_steps = get_path(&exploration, &["bounds", "max_steps"])
                .and_then(YamlValue::as_u64)
                .map(|value| value as usize);
            let max_schedules = get_path(&exploration, &["bounds", "max_schedules"])
                .and_then(YamlValue::as_u64)
                .map(|value| value as usize);
            let max_states = get_path(&exploration, &["bounds", "max_states"])
                .and_then(YamlValue::as_u64)
                .map(|value| value as usize);
            let guided_id = format!("{module}:guided-{}", analysis_index + 1);
            lanes.push(HuntLane {
                report: lane_report(
                    guided_id,
                    &module,
                    guided_property,
                    "probe-exploration",
                    "guided-semantic-novelty-v1",
                    budget_seconds,
                    format!(
                        "guided probe exploration {} (seeded, at most {MAX_GUIDED_FINDINGS} distinct findings)",
                        assembly.display()
                    ),
                    None,
                ),
                execution: LaneExecution::GuidedProbe {
                    root: checkout.to_path_buf(),
                    source_root: source_root.to_path_buf(),
                    assembly: assembly.clone(),
                    max_steps,
                    max_schedules,
                    max_states,
                },
            });
            if !has_executable_expression {
                continue;
            }
            let mut arguments = vec![
                "property".to_string(),
                "analyze".to_string(),
                relative_implementation.display().to_string(),
                "--assembly".to_string(),
                assembly
                    .strip_prefix(checkout)
                    .unwrap_or(&assembly)
                    .display()
                    .to_string(),
            ];
            for (flag, field) in [
                ("--max-steps", "max_steps"),
                ("--max-schedules", "max_schedules"),
                ("--max-states", "max_states"),
            ] {
                if let Some(bound) =
                    get_path(&exploration, &["bounds", field]).and_then(YamlValue::as_u64)
                {
                    arguments.push(flag.to_string());
                    arguments.push(bound.to_string());
                }
            }
            let analysis = run_root.join("analyses").join(format!(
                "{}-relationships-{}.yaml",
                sanitize_segment(&module),
                analysis_index + 1
            ));
            arguments.push("--out".to_string());
            arguments.push(analysis.display().to_string());
            arguments.push("--timeout-seconds".to_string());
            arguments.push(budget_seconds.to_string());
            lanes.push(HuntLane {
                report: lane_report(
                    format!("{module}:relationships-{}", analysis_index + 1),
                    &module,
                    None,
                    "relationship-analysis",
                    "finite-model-analysis",
                    budget_seconds,
                    rms_command(&arguments),
                    None,
                ),
                execution: LaneExecution::Rms {
                    root: checkout.to_path_buf(),
                    arguments,
                },
            });
        }
    }
    lanes.sort_by(|left, right| {
        lane_phase(left)
            .cmp(&lane_phase(right))
            .then_with(|| left.report.id.cmp(&right.report.id))
    });
    Ok(lanes)
}

fn property_has_executable_expression(property: &YamlValue) -> bool {
    ["temporal", "step"]
        .iter()
        .any(|field| get_path(property, &[*field]).is_some_and(|value| !value.is_null()))
}

fn selected_module_closure(
    checkout: &Path,
    selected: &Path,
) -> Result<std::collections::BTreeSet<String>> {
    let selected_path = checkout.join(selected);
    let selected_value = load_yaml_or_json(&selected_path)?;
    let selected_name = get_str(&selected_value, &["module", "name"])
        .or_else(|| get_str(&selected_value, &["module"]))
        .ok_or_else(|| anyhow!("selected hunt module has no module name"))?
        .to_string();
    let mut dependencies = BTreeMap::<String, Vec<String>>::new();
    for entry in WalkDir::new(checkout)
        .max_depth(12)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| {
            !matches!(
                entry.file_name().to_str(),
                Some(".git") | Some("target") | Some("node_modules") | Some(".rms")
            )
        })
        .filter_map(std::result::Result::ok)
        .filter(|entry| entry.file_type().is_file() && entry.file_name() == "module.yaml")
    {
        let Ok(value) = load_yaml_or_json(entry.path()) else {
            continue;
        };
        let Some(name) = get_str(&value, &["module", "name"]) else {
            continue;
        };
        let required = get_path(&value, &["requires", "modules"])
            .and_then(YamlValue::as_sequence)
            .into_iter()
            .flatten()
            .filter_map(|item| {
                item.as_str()
                    .or_else(|| get_str(item, &["name"]))
                    .map(ToString::to_string)
            })
            .collect::<Vec<_>>();
        dependencies.insert(name.to_string(), required);
    }
    let mut closure = std::collections::BTreeSet::from([selected_name]);
    let mut pending = closure.iter().cloned().collect::<Vec<_>>();
    while let Some(module) = pending.pop() {
        for dependency in dependencies.get(&module).into_iter().flatten() {
            if closure.insert(dependency.clone()) {
                pending.push(dependency.clone());
            }
        }
    }
    Ok(closure)
}

fn tool_identities(lanes: &mut [HuntLane]) -> BTreeMap<String, String> {
    let mut tools = BTreeMap::new();
    let rms_identity = std::env::current_exe()
        .ok()
        .and_then(|path| fs::read(path).ok())
        .map(|bytes| {
            format!(
                "{} sha256:{}",
                env!("CARGO_PKG_VERSION"),
                sha256_bytes(&bytes)
            )
        })
        .unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_string());
    tools.insert("rms".to_string(), rms_identity);
    for lane in lanes {
        let LaneExecution::Project { root, command, .. } = &lane.execution else {
            continue;
        };
        let Some(program) = command
            .split_whitespace()
            .find(|token| !token.contains('='))
            .map(|token| token.trim_matches(['\'', '"']))
            .filter(|token| !token.is_empty())
        else {
            lane.report.status = "invalid".to_string();
            lane.report.diagnostic = Some("declared lane command is empty".to_string());
            continue;
        };
        if tools.contains_key(program) {
            continue;
        }
        let lookup = format!("command -v {}", shell_quote(program));
        let Ok(resolved) = execute_proof_command(root, &lookup, &[], 5) else {
            lane.report.status = "unsupported".to_string();
            lane.report.diagnostic = Some(format!("required tool `{program}` is unavailable"));
            tools.insert(program.to_string(), "unavailable".to_string());
            continue;
        };
        if !resolved.status.success() {
            lane.report.status = "unsupported".to_string();
            lane.report.diagnostic = Some(format!("required tool `{program}` is unavailable"));
            tools.insert(program.to_string(), "unavailable".to_string());
            continue;
        }
        let version_command = format!("{} --version", shell_quote(program));
        let version = execute_proof_command(root, &version_command, &[], 5)
            .ok()
            .filter(|output| output.status.success())
            .map(|output| trim_output(&output.stdout, &output.stderr))
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "version unavailable".to_string());
        tools.insert(
            program.to_string(),
            format!("{} ({version})", resolved.stdout.trim()),
        );
    }
    tools
}

fn execute_lane(
    lane: &HuntLane,
    run_id: &str,
    seed: u64,
    run_root: &Path,
    hunt_deadline: Instant,
    cancellation: Arc<AtomicBool>,
) -> Result<(HuntLaneReport, Vec<HuntFinding>)> {
    let mut report = lane.report.clone();
    let started = Instant::now();
    let lane_deadline = started
        .checked_add(Duration::from_secs(report.budget_seconds))
        .map(|deadline| deadline.min(hunt_deadline))
        .unwrap_or(hunt_deadline);
    if cancellation.load(AtomicOrdering::Acquire)
        || remaining_timeout_seconds(lane_deadline).is_none()
    {
        report.status = "inconclusive".to_string();
        report.diagnostic = Some("hunt deadline exhausted before lane execution".to_string());
        return Ok((report, Vec::new()));
    }
    let lane_output = run_root
        .join("lanes")
        .join(format!("{}.yaml", sanitize_segment(&report.id)));
    if let Some(parent) = lane_output.parent() {
        fs::create_dir_all(parent)?;
    }
    let output_display = lane_output.display().to_string();
    let seed_string = seed.to_string();
    let budget_string = report.budget_seconds.to_string();
    let output = match &lane.execution {
        LaneExecution::Rms { root, arguments } => {
            let executable = std::env::current_exe()?;
            let command = format!(
                "{} {}",
                shell_quote(&executable.display().to_string()),
                arguments
                    .iter()
                    .map(|argument| shell_quote(argument))
                    .collect::<Vec<_>>()
                    .join(" ")
            );
            execute_proof_command(
                root,
                &command,
                &[],
                remaining_timeout_seconds(lane_deadline)
                    .ok_or_else(|| anyhow!("hunt lane deadline exhausted"))?,
            )?
        }
        LaneExecution::Project {
            root,
            command,
            property,
            runner,
            generator,
        } => execute_proof_command(
            root,
            command,
            &[
                ("RMS_PROPERTY_ID", property),
                ("RMS_PROPERTY_RUNNER", runner),
                ("RMS_PROPERTY_GENERATOR", generator.as_deref().unwrap_or("")),
                ("RMS_HUNT_RUN_ID", run_id),
                ("RMS_HUNT_SEED", &seed_string),
                ("RMS_HUNT_BUDGET_SECONDS", &budget_string),
                ("RMS_HUNT_OUTPUT", &output_display),
            ],
            remaining_timeout_seconds(lane_deadline)
                .ok_or_else(|| anyhow!("hunt lane deadline exhausted"))?,
        )?,
        LaneExecution::GuidedProbe {
            root,
            source_root,
            assembly,
            max_steps,
            max_schedules,
            max_states,
        } => {
            let artifact_dir = run_root
                .join("counterexamples")
                .join(sanitize_segment(&report.id));
            let guided = super::probe::run_guided_hunt(
                assembly,
                seed,
                MAX_GUIDED_FINDINGS,
                *max_steps,
                *max_schedules,
                *max_states,
                report.budget_seconds,
                root,
                source_root,
                &artifact_dir,
                &lane_output,
                lane_deadline,
                Arc::clone(&cancellation),
            );
            if let Err(error) = guided {
                report.elapsed_ms = started.elapsed().as_millis() as u64;
                if cancellation.load(AtomicOrdering::Acquire)
                    || remaining_timeout_seconds(lane_deadline).is_none()
                {
                    report.status = "inconclusive".to_string();
                    report.diagnostic = Some(format!("hunt lane deadline exhausted: {error}"));
                    report
                        .metrics
                        .insert("executed".to_string(), JsonValue::Bool(true));
                    return Ok((report, Vec::new()));
                }
                return Err(error);
            }
            let Some(timeout) = remaining_timeout_seconds(lane_deadline) else {
                report.elapsed_ms = started.elapsed().as_millis() as u64;
                report.status = "inconclusive".to_string();
                report.diagnostic = Some(
                    "hunt lane deadline exhausted after guided exploration; lane output preserved"
                        .to_string(),
                );
                return Ok((report, Vec::new()));
            };
            execute_proof_command(root, "true", &[], timeout)?
        }
    };
    report.elapsed_ms = started.elapsed().as_millis() as u64;
    if output.timed_out {
        report.status = "inconclusive".to_string();
        report.diagnostic = Some("lane budget exhausted".to_string());
        report
            .metrics
            .insert("executed".to_string(), JsonValue::Bool(true));
        return Ok((report, Vec::new()));
    }
    let mut findings = Vec::new();
    if !output.status.success() {
        report.status = if matches!(report.kind.as_str(), "baseline" | "historical-replay") {
            "finding"
        } else {
            "invalid"
        }
        .to_string();
        report.diagnostic = Some(trim_output(&output.stderr, &output.stdout));
        if report.status == "finding" {
            findings.push(HuntFinding::raw(
                report.id.clone(),
                "behavioral-counterexample".to_string(),
                if report.kind == "historical-replay" {
                    "a historical counterexample still reproduces".to_string()
                } else {
                    "the smoke baseline found a reproducible behavioral failure".to_string()
                },
                None,
                Some(report.command.clone()),
            ));
        }
    } else {
        report.status = "pass".to_string();
        if report.kind == "historical-replay"
            && serde_json::from_str::<JsonValue>(&output.stdout)
                .ok()
                .and_then(|value| value.get("evaluations").cloned())
                .and_then(|value| value.as_array().cloned())
                .is_some_and(|evaluations| {
                    evaluations.iter().any(|evaluation| {
                        evaluation.get("verdict").and_then(JsonValue::as_str) == Some("violated")
                    })
                })
        {
            report.status = "finding".to_string();
            report.diagnostic =
                Some("a historical property counterexample still reproduces".to_string());
            findings.push(HuntFinding::raw(
                report.id.clone(),
                "behavioral-counterexample".to_string(),
                "a historical property counterexample still reproduces".to_string(),
                None,
                Some(report.command.clone()),
            ));
        }
    }
    if lane_output.is_file() {
        let value = load_yaml_or_json(&lane_output)?;
        validate_schema(
            &value,
            include_str!("../../../../schemas/hunt-lane-result.schema.json"),
            &format!("lane `{}` result", report.id),
        )?;
        report.status = get_str(&value, &["status"])
            .unwrap_or("invalid")
            .to_string();
        report.metrics = get_path(&value, &["metrics"])
            .and_then(|value| serde_json::to_value(value).ok())
            .and_then(|value| value.as_object().cloned())
            .unwrap_or_default()
            .into_iter()
            .collect();
        report.artifacts = get_path(&value, &["artifacts"])
            .and_then(YamlValue::as_sequence)
            .into_iter()
            .flatten()
            .filter_map(YamlValue::as_str)
            .map(ToString::to_string)
            .collect();
        for finding in get_path(&value, &["findings"])
            .and_then(YamlValue::as_sequence)
            .into_iter()
            .flatten()
        {
            let candidate = HuntFinding::raw(
                report.id.clone(),
                get_str(finding, &["kind"]).unwrap_or("invalid").to_string(),
                get_str(finding, &["summary"])
                    .unwrap_or("hunt lane reported a finding")
                    .to_string(),
                get_str(finding, &["artifact"]).map(ToString::to_string),
                get_str(finding, &["replay"]).map(ToString::to_string),
            );
            if behavioral_finding(&candidate.kind) {
                let Some(replay) = candidate.replay.as_deref() else {
                    report.status = "invalid".to_string();
                    report.diagnostic =
                        Some("behavioral finding omitted a replay command".to_string());
                    continue;
                };
                let Some(replay_timeout) = remaining_timeout_seconds(lane_deadline) else {
                    report.status = "inconclusive".to_string();
                    report.diagnostic = Some(
                        "lane deadline exhausted before behavioral finding replay; artifact preserved"
                            .to_string(),
                    );
                    continue;
                };
                let replay_output = execute_proof_command(
                    lane_root(lane),
                    replay,
                    &[("RMS_HUNT_RUN_ID", run_id), ("RMS_HUNT_SEED", &seed_string)],
                    replay_timeout.min(120),
                )?;
                if replay_output.timed_out || !replay_succeeded(&replay_output) {
                    report.status = "invalid".to_string();
                    report.diagnostic = Some(format!(
                        "behavioral finding did not replay: {}",
                        trim_output(&replay_output.stderr, &replay_output.stdout)
                    ));
                    continue;
                }
            }
            findings.push(candidate);
        }
        if report.status == "finding" && findings.is_empty() {
            report.status = "invalid".to_string();
            report.diagnostic = Some("lane reported `finding` without a typed finding".to_string());
        } else if !findings.is_empty() && report.status == "pass" {
            report.status = "finding".to_string();
        }
    } else if report.kind == "declared-realization" {
        report.status = "invalid".to_string();
        report.diagnostic = Some(format!(
            "nightly runner did not write {} to RMS_HUNT_OUTPUT",
            LANE_RESULT_SPEC
        ));
    } else if matches!(
        report.kind.as_str(),
        "probe-exploration" | "trace-evaluation" | "relationship-analysis"
    ) {
        if let LaneExecution::Rms { arguments, .. } = &lane.execution {
            if let Some(index) = arguments.iter().position(|argument| argument == "--out") {
                if let Some(path) = arguments.get(index + 1) {
                    report.artifacts.push(path.clone());
                    if let Ok(value) = load_yaml_or_json(Path::new(path)) {
                        let result = get_str(&value, &["result"]).unwrap_or("invalid");
                        if let Some(exhausted) = get_path(&value, &["coverage", "exhausted"])
                            .and_then(YamlValue::as_bool)
                        {
                            report
                                .metrics
                                .insert("exhausted".to_string(), JsonValue::Bool(exhausted));
                        }
                        if result == "violated-counterexample"
                            || (report.kind == "trace-evaluation" && result == "violated")
                        {
                            let Some(replay_timeout) = remaining_timeout_seconds(lane_deadline)
                            else {
                                report.status = "inconclusive".to_string();
                                report.diagnostic = Some(
                                    "lane deadline exhausted before property replay; analysis preserved"
                                        .to_string(),
                                );
                                report
                                    .metrics
                                    .insert("executed".to_string(), JsonValue::Bool(true));
                                enrich_lane_findings(&report, lane_root(lane), &mut findings);
                                return Ok((report, findings));
                            };
                            let replay = replay_property_analysis(
                                lane_root(lane),
                                Path::new(path),
                                replay_timeout.min(120),
                            )?;
                            if replay.status.success() && !replay.timed_out {
                                report.status = "finding".to_string();
                                findings.push(HuntFinding::raw(
                                    report.id.clone(),
                                    "behavioral-counterexample".to_string(),
                                    if report.kind == "trace-evaluation" {
                                        "real-trace evaluation found a replayable property violation"
                                            .to_string()
                                    } else {
                                        "probe exploration found a replayable property violation"
                                            .to_string()
                                    },
                                    Some(path.clone()),
                                    Some(format!(
                                        "rms property replay {}",
                                        shell_quote(path)
                                    )),
                                ));
                            } else {
                                report.status = "invalid".to_string();
                                report.diagnostic = Some(format!(
                                    "property failure did not replay: {}",
                                    trim_output(&replay.stderr, &replay.stdout)
                                ));
                            }
                        } else if result == "inconclusive" {
                            report.status = "inconclusive".to_string();
                        } else if report.kind == "relationship-analysis" {
                            let relationships = get_path(&value, &["relationships"])
                                .and_then(YamlValue::as_sequence)
                                .into_iter()
                                .flatten()
                                .collect::<Vec<_>>();
                            report.metrics.insert(
                                "relationships".to_string(),
                                JsonValue::from(relationships.len() as u64),
                            );
                            report.metrics.insert(
                                "refuted_relationships".to_string(),
                                JsonValue::from(
                                    relationships
                                        .iter()
                                        .filter(|relationship| {
                                            get_str(relationship, &["result"]) == Some("refuted")
                                        })
                                        .count() as u64,
                                ),
                            );
                            report.status = "pass".to_string();
                        } else if result == "invalid" {
                            report.status = "invalid".to_string();
                        } else {
                            report.status = "pass".to_string();
                        }
                    }
                }
            }
        }
    }
    report
        .metrics
        .insert("executed".to_string(), JsonValue::Bool(true));
    enrich_lane_findings(&report, lane_root(lane), &mut findings);
    Ok((report, findings))
}

fn enrich_lane_findings(report: &HuntLaneReport, root: &Path, findings: &mut [HuntFinding]) {
    for finding in findings {
        if finding.module.is_empty() {
            finding.module = report.module.clone();
        }
        if finding.property.is_none() {
            finding.property = report.property.clone();
        }
        if let Some(artifact) = finding.artifact.as_deref() {
            let path = if Path::new(artifact).is_absolute() {
                PathBuf::from(artifact)
            } else {
                root.join(artifact)
            };
            if let Ok(value) = load_yaml_or_json(&path) {
                finding.check = finding
                    .check
                    .take()
                    .or_else(|| get_str(&value, &["failure", "check"]).map(ToString::to_string))
                    .or_else(|| {
                        get_str(&value, &["counterexample", "failure", "check"])
                            .map(ToString::to_string)
                    });
                finding.first_bad_transition = finding
                    .first_bad_transition
                    .take()
                    .or_else(|| first_bad_transition(&value));
            }
        }
        if finding.id.is_empty() {
            finding.id = stable_finding_id(finding);
        }
        finding.occurrences = finding.occurrences.max(1);
    }
}

fn first_bad_transition(value: &YamlValue) -> Option<String> {
    for path in [
        &["trace", "timeline"][..],
        &["evidence_trace", "timeline"][..],
        &["timeline"][..],
    ] {
        if let Some(timeline) = get_path(value, path).and_then(YamlValue::as_sequence) {
            if let Some(case) = timeline
                .iter()
                .rev()
                .find_map(|entry| get_str(entry, &["transition_case"]))
            {
                return Some(case.to_string());
            }
        }
    }
    None
}

fn stable_finding_id(finding: &HuntFinding) -> String {
    let property = finding.property.as_deref().unwrap_or("unscoped");
    let identity = serde_json::json!({
        "module": finding.module,
        "property": property,
        "kind": finding.kind,
        "check": finding.check,
        "first_bad_transition_fallback": if finding.check.is_none() {
            finding.first_bad_transition.as_deref()
        } else {
            None
        },
        "lane_fallback": (finding.check.is_none() && finding.first_bad_transition.is_none())
            .then_some(finding.lane.as_str()),
    });
    let digest = sha256_bytes(&serde_json::to_vec(&identity).unwrap_or_default());
    format!(
        "{}/{}/{}",
        sanitize_segment(if finding.module.is_empty() {
            "unknown"
        } else {
            &finding.module
        }),
        sanitize_segment(property),
        &digest[..12]
    )
}

fn normalize_report_findings(report: &mut HuntReport) {
    let lanes = report
        .lanes
        .iter()
        .map(|lane| (lane.id.as_str(), lane))
        .collect::<BTreeMap<_, _>>();
    for finding in &mut report.findings {
        if let Some(lane) = lanes.get(finding.lane.as_str()) {
            if finding.module.is_empty() {
                finding.module = lane.module.clone();
            }
            if finding.property.is_none() {
                finding.property = lane.property.clone();
            }
        }
        if finding.id.is_empty() {
            finding.id = stable_finding_id(finding);
        }
        finding.occurrences = finding.occurrences.max(1);
    }

    let mut grouped = BTreeMap::<String, HuntFinding>::new();
    for finding in std::mem::take(&mut report.findings) {
        let cost = finding_replay_cost(&finding);
        match grouped.entry(finding.id.clone()) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(finding);
            }
            std::collections::btree_map::Entry::Occupied(mut entry) => {
                let existing = entry.get_mut();
                let occurrences = existing.occurrences.saturating_add(finding.occurrences);
                if cost < finding_replay_cost(existing) {
                    let mut replacement = finding;
                    replacement.occurrences = occurrences;
                    *existing = replacement;
                } else {
                    existing.occurrences = occurrences;
                }
            }
        }
    }
    report.findings = grouped.into_values().collect();
    report.findings.sort_by(|left, right| {
        left.module
            .cmp(&right.module)
            .then(left.property.cmp(&right.property))
            .then(left.kind.cmp(&right.kind))
            .then(left.first_bad_transition.cmp(&right.first_bad_transition))
            .then(left.id.cmp(&right.id))
    });
}

fn finding_replay_cost(finding: &HuntFinding) -> usize {
    let Some(path) = finding.artifact.as_deref() else {
        return usize::MAX;
    };
    load_yaml_or_json(Path::new(path))
        .ok()
        .and_then(|value| {
            get_path(&value, &["decisions"])
                .and_then(YamlValue::as_sequence)
                .map(Vec::len)
                .or_else(|| {
                    get_path(&value, &["trace", "timeline"])
                        .and_then(YamlValue::as_sequence)
                        .map(Vec::len)
                })
        })
        .unwrap_or(usize::MAX)
}

fn behavioral_finding(kind: &str) -> bool {
    matches!(
        kind,
        "behavioral-counterexample"
            | "crash"
            | "sanitizer-failure"
            | "deadlock"
            | "relationship-refutation"
    )
}

fn replay_succeeded(output: &super::ProofProcessOutput) -> bool {
    output.status.success()
        || serde_json::from_str::<JsonValue>(&output.stdout)
            .ok()
            .and_then(|value| {
                value
                    .get("result")
                    .and_then(JsonValue::as_str)
                    .map(ToString::to_string)
            })
            .is_some_and(|result| matches!(result.as_str(), "reproduced" | "replayed"))
}

fn replay_property_analysis(
    root: &Path,
    analysis: &Path,
    timeout_seconds: u64,
) -> Result<super::ProofProcessOutput> {
    let executable = std::env::current_exe()?;
    let command = format!(
        "{} property replay {} --json",
        shell_quote(&executable.display().to_string()),
        shell_quote(&analysis.display().to_string())
    );
    execute_proof_command(root, &command, &[], timeout_seconds)
}

fn lane_root(lane: &HuntLane) -> &Path {
    match &lane.execution {
        LaneExecution::Rms { root, .. }
        | LaneExecution::Project { root, .. }
        | LaneExecution::GuidedProbe { root, .. } => root,
    }
}

fn lane_phase(lane: &HuntLane) -> u8 {
    match lane.report.kind.as_str() {
        "historical-replay" => 0,
        "baseline" => 1,
        "trace-regeneration" => 2,
        "trace-evaluation" => 3,
        "probe-exploration" | "relationship-analysis" => 5,
        _ if matches!(
            lane.report.strategy.as_str(),
            "deterministic-exhaustive" | "model-checker" | "static-analyzer" | "sanitizer"
        ) =>
        {
            4
        }
        _ if lane.report.strategy == "mutation-tester" => 6,
        _ => 7,
    }
}

fn phase_parallelism(phase: u8, jobs: usize) -> usize {
    if matches!(phase, 0 | 2 | 6) {
        1
    } else {
        jobs
    }
}

fn finish_report(
    root: &Path,
    worktree: &Path,
    run_root: &Path,
    out: Option<&Path>,
    report: &HuntReport,
) -> Result<()> {
    write_report(&run_root.join("checkpoint.yaml"), report)?;
    write_report(&run_root.join("report.yaml"), report)?;
    if let Some(out) = out {
        let output = if out.is_absolute() {
            out.to_path_buf()
        } else {
            root.join(out)
        };
        write_report(&output, report)?;
    }
    remove_isolated_checkout(root, worktree);
    Ok(())
}

fn hunt_result(report: &HuntReport) -> String {
    let bug_kinds = [
        "behavioral-counterexample",
        "crash",
        "sanitizer-failure",
        "deadlock",
        "relationship-refutation",
    ];
    if report
        .findings
        .iter()
        .any(|finding| bug_kinds.contains(&finding.kind.as_str()))
    {
        "bugs-found"
    } else if !report.findings.is_empty() {
        "proof-gaps-found"
    } else if report.lanes.is_empty()
        || report.lanes.iter().all(|lane| lane.status == "unsupported")
    {
        "unsupported"
    } else if report.lanes.iter().any(|lane| lane.status == "invalid") {
        "invalid"
    } else if report
        .lanes
        .iter()
        .any(|lane| matches!(lane.status.as_str(), "planned" | "running" | "inconclusive"))
    {
        "inconclusive"
    } else {
        "clean-under-recorded-bounds"
    }
    .to_string()
}

fn proof_scope(lanes: &[HuntLaneReport]) -> HuntProofScope {
    let mut exhausted_lanes = Vec::new();
    let mut bounded_lanes = Vec::new();
    let mut unsupported_lanes = Vec::new();
    let mut unexecuted_lanes = 0usize;
    for lane in lanes {
        if lane.status == "unsupported" {
            unsupported_lanes.push(lane.id.clone());
        } else if lane.status == "pass"
            && matches!(lane.metrics.get("exhausted"), Some(JsonValue::Bool(true)))
            && matches!(
                lane.strategy.as_str(),
                "deterministic-exhaustive" | "finite-model-analysis" | "probe-search"
            )
        {
            exhausted_lanes.push(lane.id.clone());
        } else if matches!(lane.metrics.get("executed"), Some(JsonValue::Bool(true))) {
            bounded_lanes.push(lane.id.clone());
        } else {
            unexecuted_lanes += 1;
        }
    }
    let unexecuted = if unexecuted_lanes == 0 {
        String::new()
    } else {
        format!(" {unexecuted_lanes} planned lane(s) did not execute and contribute no evidence.")
    };
    HuntProofScope {
        claim: format!(
            "No global bug-free claim is made; exhausted lanes prove only their declared finite model, while executed non-exhaustive lanes are bounded evidence.{unexecuted}"
        ),
        exhausted_lanes,
        bounded_lanes,
        unsupported_lanes,
    }
}

fn empty_scope() -> HuntProofScope {
    HuntProofScope {
        claim: "No global bug-free claim is made.".to_string(),
        exhausted_lanes: Vec::new(),
        bounded_lanes: Vec::new(),
        unsupported_lanes: Vec::new(),
    }
}

fn print_report(report: &HuntReport, json_output: bool) -> Result<()> {
    if json_output {
        println!("{}", serde_json::to_string_pretty(report)?);
    } else {
        println!("RMS hunt: {}", report.result);
        println!("run: {}", report.run_id);
        println!("source: {}", report.source.revision);
        if report.lanes.iter().all(|lane| lane.status == "planned") {
            let strategies = report
                .lanes
                .iter()
                .map(|lane| lane.strategy.as_str())
                .collect::<std::collections::BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>()
                .join(", ");
            let properties = report
                .lanes
                .iter()
                .filter_map(|lane| lane.property.as_deref())
                .collect::<std::collections::BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>()
                .join(", ");
            println!("will vary: {strategies}");
            println!(
                "will check: {}",
                if properties.is_empty() {
                    "declared baselines, assembly checks, and relationships"
                } else {
                    &properties
                }
            );
            println!(
                "limits: {}s total, seed {}, {} worker(s); guided exploration remains bounded evidence",
                report.configuration.budget_seconds,
                report.configuration.seed,
                report.configuration.jobs
            );
            let guided_lanes = report
                .lanes
                .iter()
                .filter(|lane| lane.strategy == "guided-semantic-novelty-v1")
                .count();
            println!("guided lanes: {guided_lanes}");
            if guided_lanes == 0 {
                println!(
                    "guided setup: declare properties[].explorations[].assembly on the selected implementation"
                );
            }
        }
        println!("findings: {} unique", report.findings.len());
        for (index, finding) in report.findings.iter().enumerate() {
            println!("  {}. {} [{}]", index + 1, finding.summary, finding.id);
            if let Some(property) = &finding.property {
                println!("     property: {property}");
            }
            if let Some(case) = &finding.first_bad_transition {
                println!("     first bad transition: {case}");
            }
            if finding.occurrences > 1 {
                println!("     occurrences: {}", finding.occurrences);
            }
            if let Some(replay) = &finding.replay {
                println!("     replay: {replay}");
            }
        }
        println!("lanes: {}", report.lanes.len());
        println!("claim: {}", report.proof_scope.claim);
    }
    Ok(())
}

fn load_resume_report(
    hunts_root: &Path,
    resume: Option<&str>,
) -> Result<Option<(String, HuntReport)>> {
    if let Some(resume) = resume {
        let run_id = if resume == "latest" {
            let mut directories = fs::read_dir(hunts_root)?
                .filter_map(std::result::Result::ok)
                .filter(|entry| entry.path().is_dir())
                .map(|entry| entry.file_name().to_string_lossy().to_string())
                .collect::<Vec<_>>();
            directories.sort();
            directories
                .pop()
                .ok_or_else(|| anyhow!("no hunt run is available to resume"))?
        } else {
            resume.to_string()
        };
        let checkpoint = hunts_root.join(&run_id).join("checkpoint.yaml");
        let report: HuntReport = serde_yaml::from_str(&fs::read_to_string(&checkpoint)?)?;
        return Ok(Some((run_id, report)));
    }
    Ok(None)
}

fn validate_resume(
    report: &HuntReport,
    requested_seed: Option<u64>,
    revision: &str,
    declaration_digest: &str,
) -> Result<()> {
    if report.source.revision != revision || report.source.declaration_digest != declaration_digest
    {
        bail!("hunt resume rejected source or declaration drift");
    }
    if requested_seed.is_some_and(|seed| seed != report.configuration.seed) {
        bail!("hunt resume rejected seed drift");
    }
    Ok(())
}

fn generate_seed() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64
        ^ std::process::id() as u64
}

fn new_run_id(revision: &str) -> String {
    format!("{}-{}", now_ms(), &revision[..revision.len().min(12)])
}

fn ensure_resume_tool_identities(
    previous: &HuntReport,
    current: &BTreeMap<String, String>,
) -> Result<()> {
    if previous.configuration.tools != *current {
        bail!("hunt resume rejected tool identity drift");
    }
    Ok(())
}

fn ensure_clean_commit(root: &Path) -> Result<()> {
    let status = git_output(root, &["status", "--porcelain", "--untracked-files=normal"])?;
    if !status.trim().is_empty() {
        let paths = status
            .lines()
            .take(20)
            .map(str::trim)
            .collect::<Vec<_>>()
            .join("\n  ");
        bail!("`rms hunt` requires a clean committed checkout; changed paths:\n  {paths}");
    }
    Ok(())
}

fn declaration_digest(
    root: &Path,
    selected_module: Option<&Path>,
    selected_assembly: Option<&Path>,
) -> Result<String> {
    let mut files = WalkDir::new(root)
        .max_depth(12)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| {
            !matches!(
                entry.file_name().to_str(),
                Some(".git") | Some("target") | Some("node_modules") | Some(".rms")
            )
        })
        .filter_map(std::result::Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .map(|entry| entry.into_path())
        .filter(|path| {
            matches!(
                path.file_name().and_then(|name| name.to_str()),
                Some("module.yaml") | Some("implementation.yaml")
            )
        })
        .collect::<Vec<_>>();
    if let Some(module) = selected_module {
        let closure = selected_module_closure(root, module)?;
        files.retain(|path| {
            load_yaml_or_json(path)
                .ok()
                .and_then(|value| {
                    get_str(&value, &["module", "name"])
                        .or_else(|| get_str(&value, &["module"]))
                        .map(ToString::to_string)
                })
                .is_some_and(|name| closure.contains(&name))
        });
        if files.is_empty() {
            bail!(
                "selected hunt module `{}` resolved to an empty declaration closure",
                module.display()
            );
        }
    }
    if let Some(assembly) = selected_assembly {
        files.push(root.join(assembly));
    }
    files.sort();
    files.dedup();
    let mut bytes = Vec::new();
    for path in files {
        bytes.extend_from_slice(
            path.strip_prefix(root)
                .unwrap_or(&path)
                .as_os_str()
                .as_encoded_bytes(),
        );
        bytes.extend_from_slice(&fs::read(path)?);
    }
    Ok(sha256_bytes(&bytes))
}

fn prepare_isolated_checkout(root: &Path, worktree: &Path, revision: &str) -> Result<()> {
    if worktree.exists() {
        remove_isolated_checkout(root, worktree);
    }
    let output = Command::new("git")
        .current_dir(root)
        .args(["worktree", "add", "--detach"])
        .arg(worktree)
        .arg(revision)
        .output()?;
    if !output.status.success() {
        bail!(
            "failed to create isolated hunt checkout: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(())
}

fn remove_isolated_checkout(root: &Path, worktree: &Path) {
    if worktree.exists() {
        let _ = Command::new("git")
            .current_dir(root)
            .args(["worktree", "remove", "--force"])
            .arg(worktree)
            .output();
    }
}

fn resolve_hunt_file(root: &Path, path: &Path, kind: &str) -> Result<PathBuf> {
    let mut candidates = Vec::new();
    if path.is_absolute() {
        candidates.push(path.to_path_buf());
    } else {
        candidates.push(root.join(path));
        if let Ok(repository) = git_output(root, &["rev-parse", "--show-toplevel"]) {
            let repository_candidate = PathBuf::from(repository).join(path);
            if !candidates.contains(&repository_candidate) {
                candidates.push(repository_candidate);
            }
        }
        if let Ok(current) = std::env::current_dir() {
            let current_candidate = current.join(path);
            if !candidates.contains(&current_candidate) {
                candidates.push(current_candidate);
            }
        }
    }
    let absolute = candidates
        .iter()
        .find(|candidate| candidate.is_file())
        .ok_or_else(|| {
            anyhow!(
                "hunt {kind} `{}` was not found relative to the hunt root, repository root, or current directory",
                path.display()
            )
        })?
        .canonicalize()?;
    absolute
        .strip_prefix(root)
        .map(Path::to_path_buf)
        .map_err(|_| anyhow!("hunt {kind} must be inside the hunt root"))
}

fn absolute_output_path(root: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    }
}

fn git_output(root: &Path, arguments: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .current_dir(root)
        .args(arguments)
        .output()?;
    if !output.status.success() {
        bail!(
            "git {} failed: {}",
            arguments.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn parse_duration_seconds(value: &str) -> Result<u64> {
    let split = value
        .find(|character: char| !character.is_ascii_digit())
        .unwrap_or(value.len());
    let (number, unit) = value.split_at(split);
    let number = number
        .parse::<u64>()
        .with_context(|| format!("invalid hunt budget `{value}`"))?;
    let multiplier = match unit {
        "s" | "" => 1,
        "m" => 60,
        "h" => 3600,
        "d" => 86_400,
        _ => bail!("invalid hunt budget unit `{unit}`; use s, m, h, or d"),
    };
    number
        .checked_mul(multiplier)
        .filter(|seconds| *seconds > 0)
        .ok_or_else(|| anyhow!("hunt budget must be positive and finite"))
}

#[allow(clippy::too_many_arguments)]
fn lane_report(
    id: String,
    module: &str,
    property: Option<String>,
    kind: &str,
    strategy: &str,
    budget_seconds: u64,
    command: String,
    runner: Option<String>,
) -> HuntLaneReport {
    HuntLaneReport {
        id,
        module: module.to_string(),
        property,
        kind: kind.to_string(),
        strategy: strategy.to_string(),
        status: "planned".to_string(),
        budget_seconds: budget_seconds.max(1),
        elapsed_ms: 0,
        command,
        runner,
        metrics: BTreeMap::new(),
        artifacts: Vec::new(),
        diagnostic: None,
    }
}

fn write_report(path: &Path, report: &HuntReport) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    validate_schema(
        &serde_yaml::to_value(report)?,
        include_str!("../../../../schemas/hunt-report-v0.2.schema.json"),
        "hunt report",
    )?;
    let contents = if path.extension().and_then(|extension| extension.to_str()) == Some("json") {
        format!("{}\n", serde_json::to_string_pretty(report)?)
    } else {
        serde_yaml::to_string(report)?
    };
    fs::write(path, contents)?;
    Ok(())
}

fn validate_schema(value: &YamlValue, source: &str, label: &str) -> Result<()> {
    let schema: JsonValue = serde_json::from_str(source)?;
    let instance = serde_json::to_value(value)?;
    let validator =
        jsonschema::validator_for(&schema).with_context(|| format!("{label} schema is invalid"))?;
    if let Some(error) = validator.iter_errors(&instance).next() {
        bail!("{label} is invalid: {error}");
    }
    Ok(())
}

fn load_yaml_or_json(path: &Path) -> Result<YamlValue> {
    let source = fs::read_to_string(path)?;
    if path.extension().and_then(|extension| extension.to_str()) == Some("json") {
        let value: JsonValue = serde_json::from_str(&source)?;
        Ok(serde_yaml::to_value(value)?)
    } else {
        Ok(serde_yaml::from_str(&source)?)
    }
}

fn rms_command(arguments: &[String]) -> String {
    format!("rms {}", arguments.join(" "))
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

fn sanitize_segment(value: &str) -> String {
    let mut output = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '-' {
                character
            } else {
                '-'
            }
        })
        .collect::<String>();
    while output.contains("--") {
        output = output.replace("--", "-");
    }
    output.trim_matches('-').to_string()
}

fn trim_output(stderr: &str, stdout: &str) -> String {
    let value = if stderr.trim().is_empty() {
        stdout.trim()
    } else {
        stderr.trim()
    };
    value.chars().take(1000).collect()
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn remaining_timeout_seconds(deadline: Instant) -> Option<u64> {
    let remaining = deadline.checked_duration_since(Instant::now())?;
    if remaining.is_zero() {
        return None;
    }
    let millis = remaining.as_millis().max(1);
    Some(((millis + 999) / 1_000).min(u64::MAX as u128) as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hunt_duration_is_checked_and_exact() {
        assert_eq!(parse_duration_seconds("8h").unwrap(), 28_800);
        assert_eq!(parse_duration_seconds("30m").unwrap(), 1_800);
        assert!(parse_duration_seconds("0h").is_err());
        assert!(parse_duration_seconds("2fortnights").is_err());
    }

    #[test]
    fn selected_module_paths_are_root_or_current_directory_relative_and_digest_real_files() {
        let repository = Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(3)
            .expect("repository root")
            .canonicalize()
            .unwrap();
        let root = repository.join("examples/tic-tac-toe");
        let root_relative = Path::new("modules/tic-tac-toe-cli/module.yaml");
        let repository_relative =
            Path::new("examples/tic-tac-toe/modules/tic-tac-toe-cli/module.yaml");
        let first = resolve_hunt_file(&root, root_relative, "module").unwrap();
        let second = resolve_hunt_file(&root, repository_relative, "module").unwrap();
        assert_eq!(first, root_relative);
        assert_eq!(second, root_relative);
        assert_ne!(
            declaration_digest(&root, Some(&first), None).unwrap(),
            sha256_bytes(&[])
        );
    }

    #[test]
    fn direct_probe_assembly_plans_one_guided_lane_and_hashes_the_assembly() {
        let repository = Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(3)
            .expect("repository root")
            .canonicalize()
            .unwrap();
        let assembly = Path::new("examples/probes/public-rust-workload-failures.yaml");
        let lane = discover_direct_assembly_lane(&repository, &repository, assembly, 30).unwrap();
        assert_eq!(lane.report.strategy, "guided-semantic-novelty-v1");
        assert_eq!(lane.report.status, "planned");
        assert_ne!(
            declaration_digest(&repository, None, Some(assembly)).unwrap(),
            sha256_bytes(&[])
        );
    }

    #[test]
    fn direct_assembly_hunt_is_unsupported_when_required_owner_is_not_adopted() {
        let repository = Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(3)
            .expect("repository root")
            .canonicalize()
            .unwrap();
        let checkout = std::env::temp_dir().join(format!(
            "rms-hunt-campaign-gap-{}-{}",
            std::process::id(),
            now_ms()
        ));
        fs::create_dir_all(&checkout).unwrap();
        let assembly = checkout.join("assembly.yaml");
        let value = serde_json::json!({
            "spec": "rms/probe-assembly/v0.2",
            "instances": [{
                "id": "readiness",
                "implementation": repository.join("examples/rust/implementation.yaml")
            }],
            "stimuli": [],
            "coverage": {
                "required_modules": ["connection-media-epoch-delivery"],
                "fault_families": [{
                    "id": "direct-consent-loss",
                    "owner_module": "connection-media-epoch-delivery",
                    "generator": {"kind": "stimulus", "id": "direct-consent-loss"}
                }]
            }
        });
        fs::write(&assembly, serde_yaml::to_string(&value).unwrap()).unwrap();

        let lane =
            discover_direct_assembly_lane(&checkout, &repository, Path::new("assembly.yaml"), 30)
                .unwrap();
        assert_eq!(lane.report.status, "unsupported");
        let diagnostic = lane.report.diagnostic.unwrap();
        assert!(diagnostic
            .contains("decision owner module `connection-media-epoch-delivery` is not adopted"));
        assert!(diagnostic.contains("generator `stimulus:direct-consent-loss` that is absent"));
        fs::remove_dir_all(checkout).unwrap();
    }

    #[test]
    fn semantic_explorations_plan_guided_lanes_without_invalid_solver_lanes() {
        let root = std::env::temp_dir().join(format!(
            "rms-hunt-semantic-exploration-{}-{}",
            std::process::id(),
            now_ms()
        ));
        let module = root.join("modules/example");
        fs::create_dir_all(module.join("verification/assemblies")).unwrap();
        fs::write(
            module.join("implementation.yaml"),
            r#"spec: rms/implementation/v0.1
module: example
architecture:
  reliability:
    properties:
    - id: semantic-only
      temporal: null
      explorations:
      - assembly: verification/assemblies/semantic.yaml
        goal: violate
        bounds: { max_steps: 4, max_schedules: 8, max_states: 8 }
"#,
        )
        .unwrap();
        let lanes = discover_lanes(&root, &root, None, 30, &root.join("run")).unwrap();
        assert_eq!(lanes.len(), 1);
        assert_eq!(lanes[0].report.strategy, "guided-semantic-novelty-v1");
        assert!(!lanes.iter().any(|lane| {
            matches!(
                lane.report.strategy.as_str(),
                "probe-search" | "finite-model-analysis"
            )
        }));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn executable_property_detection_rejects_null_placeholders() {
        let semantic: YamlValue = serde_yaml::from_str("temporal: null\nstep: null\n").unwrap();
        let temporal: YamlValue = serde_yaml::from_str("temporal: {scope: machine}\n").unwrap();
        let behavioral: YamlValue = serde_yaml::from_str("step: {requires: []}\n").unwrap();
        assert!(!property_has_executable_expression(&semantic));
        assert!(property_has_executable_expression(&temporal));
        assert!(property_has_executable_expression(&behavioral));
    }

    #[test]
    fn hunt_outcomes_distinguish_bugs_gaps_and_bounds() {
        let mut report = HuntReport {
            spec: REPORT_SPEC.to_string(),
            run_id: "test".to_string(),
            result: "inconclusive".to_string(),
            source: HuntSource {
                revision: "git:test".to_string(),
                declaration_digest: "digest".to_string(),
                root: ".".to_string(),
            },
            configuration: HuntConfiguration {
                budget_seconds: 1,
                seed: 1,
                jobs: 1,
                module: None,
                assembly: None,
                output: None,
                tools: BTreeMap::new(),
            },
            lanes: vec![lane_report(
                "lane".to_string(),
                "module",
                None,
                "declared-realization",
                "coverage-fuzzer",
                1,
                "true".to_string(),
                None,
            )],
            findings: Vec::new(),
            proof_scope: empty_scope(),
            elapsed_ms: 0,
            started_at_unix_ms: 0,
            finished_at_unix_ms: None,
        };
        report.lanes[0].status = "pass".to_string();
        assert_eq!(hunt_result(&report), "clean-under-recorded-bounds");
        report.findings.push(HuntFinding::raw(
            "lane".to_string(),
            "surviving-mutant".to_string(),
            "oracle missed mutation".to_string(),
            None,
            None,
        ));
        assert_eq!(hunt_result(&report), "proof-gaps-found");
        report.findings[0].kind = "crash".to_string();
        assert_eq!(hunt_result(&report), "bugs-found");
    }

    #[test]
    fn hunt_scope_excludes_unstarted_lanes_and_baselines_use_parallelism() {
        let mut executed = lane_report(
            "executed".to_string(),
            "module",
            None,
            "baseline",
            "deterministic-baseline",
            10,
            "true".to_string(),
            None,
        );
        executed.status = "pass".to_string();
        executed
            .metrics
            .insert("executed".to_string(), JsonValue::Bool(true));
        let mut unstarted = lane_report(
            "unstarted".to_string(),
            "module",
            None,
            "baseline",
            "deterministic-baseline",
            10,
            "true".to_string(),
            None,
        );
        unstarted.status = "inconclusive".to_string();
        unstarted.diagnostic = Some("total hunt budget exhausted".to_string());

        let scope = proof_scope(&[executed, unstarted]);
        assert_eq!(scope.bounded_lanes, vec!["executed"]);
        assert!(!scope.bounded_lanes.contains(&"unstarted".to_string()));
        assert!(scope.claim.contains("1 planned lane(s) did not execute"));
        assert_eq!(phase_parallelism(1, 4), 4);
        assert_eq!(phase_parallelism(2, 4), 1);
        assert_eq!(phase_parallelism(6, 4), 1);

        let mut guided = lane_report(
            "guided".to_string(),
            "module",
            None,
            "probe-exploration",
            "guided-semantic-novelty-v1",
            10,
            "internal".to_string(),
            None,
        );
        guided.status = "pass".to_string();
        guided
            .metrics
            .insert("executed".to_string(), JsonValue::Bool(true));
        guided
            .metrics
            .insert("exhausted".to_string(), JsonValue::Bool(true));
        let scope = proof_scope(&[guided]);
        assert!(scope.exhausted_lanes.is_empty());
        assert_eq!(scope.bounded_lanes, vec!["guided"]);
    }

    #[test]
    fn v2_findings_have_stable_ids_and_deduplicate_to_the_shortest_replay() {
        let root = std::env::temp_dir().join(format!(
            "rms-hunt-finding-{}-{}",
            std::process::id(),
            now_ms()
        ));
        fs::create_dir_all(&root).unwrap();
        let long = root.join("long.yaml");
        let short = root.join("short.yaml");
        fs::write(&long, "decisions: [{}, {}, {}]\n").unwrap();
        fs::write(&short, "decisions: [{}]\n").unwrap();
        let lane = lane_report(
            "lane-a".to_string(),
            "orders",
            Some("never-lose-order".to_string()),
            "probe-exploration",
            "guided-semantic-novelty-v1",
            1,
            "internal".to_string(),
            None,
        );
        let mut second_lane = lane.clone();
        second_lane.id = "lane-b".to_string();
        let mut first = HuntFinding::raw(
            "lane-a".to_string(),
            "behavioral-counterexample".to_string(),
            "failed".to_string(),
            Some(long.display().to_string()),
            Some("replay long".to_string()),
        );
        first.check = Some("order-completes".to_string());
        let mut second = first.clone();
        second.lane = "lane-b".to_string();
        second.artifact = Some(short.display().to_string());
        second.replay = Some("replay short".to_string());
        let mut report = HuntReport {
            spec: REPORT_SPEC.to_string(),
            run_id: "stable".to_string(),
            result: "bugs-found".to_string(),
            source: HuntSource {
                revision: "git:test".to_string(),
                declaration_digest: "sha256:test".to_string(),
                root: ".".to_string(),
            },
            configuration: HuntConfiguration {
                budget_seconds: 1,
                seed: 1,
                jobs: 1,
                module: None,
                assembly: None,
                output: None,
                tools: BTreeMap::new(),
            },
            lanes: vec![lane, second_lane],
            findings: vec![first, second],
            proof_scope: empty_scope(),
            elapsed_ms: 0,
            started_at_unix_ms: 0,
            finished_at_unix_ms: Some(1),
        };
        normalize_report_findings(&mut report);
        assert_eq!(report.findings.len(), 1);
        assert_eq!(report.findings[0].occurrences, 2);
        assert_eq!(report.findings[0].replay.as_deref(), Some("replay short"));
        assert!(report.findings[0]
            .id
            .starts_with("orders/never-lose-order/"));
        validate_schema(
            &serde_yaml::to_value(&report).unwrap(),
            include_str!("../../../../schemas/hunt-report-v0.2.schema.json"),
            "hunt report v0.2",
        )
        .unwrap();
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn hunt_schemas_accept_every_closed_outcome_and_lane_status() {
        for result in [
            "bugs-found",
            "proof-gaps-found",
            "clean-under-recorded-bounds",
            "inconclusive",
            "invalid",
            "unsupported",
        ] {
            let value: YamlValue = serde_yaml::from_str(&format!(
                r#"spec: rms/hunt-report/v0.1
run_id: schema
result: {result}
source: {{ revision: git:test, declaration_digest: sha256:test, root: . }}
configuration:
  budget_seconds: 1
  seed: 1
  jobs: 1
  module: null
  tools: {{ rms: test }}
lanes: []
findings: []
proof_scope:
  claim: No global proof.
  exhausted_lanes: []
  bounded_lanes: []
  unsupported_lanes: []
started_at_unix_ms: 0
finished_at_unix_ms: null
"#
            ))
            .unwrap();
            validate_schema(
                &value,
                include_str!("../../../../schemas/hunt-report.schema.json"),
                "hunt report",
            )
            .unwrap();
        }
        for status in ["pass", "finding", "inconclusive", "invalid", "unsupported"] {
            let value: YamlValue = serde_yaml::from_str(&format!(
                "spec: rms/hunt-lane-result/v0.1\nstatus: {status}\n"
            ))
            .unwrap();
            validate_schema(
                &value,
                include_str!("../../../../schemas/hunt-lane-result.schema.json"),
                "lane result",
            )
            .unwrap();
        }
    }

    #[test]
    fn rms_native_nonzero_reproduction_is_valid_replay_evidence() {
        let output = execute_proof_command(
            Path::new("."),
            "printf '%s' '{\"result\":\"reproduced\"}'; exit 1",
            &[],
            2,
        )
        .unwrap();
        assert!(!output.status.success());
        assert!(replay_succeeded(&output));
    }

    #[test]
    fn hunt_run_storage_prepares_analysis_output_parent() {
        let root = std::env::temp_dir().join(format!(
            "rms-hunt-storage-{}-{}",
            std::process::id(),
            now_ms()
        ));
        prepare_run_storage(&root).unwrap();
        assert!(root.join("analyses").is_dir());
        fs::write(root.join("analyses/property.yaml"), "result: pass\n").unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn hunt_resume_rejects_source_declaration_seed_and_tool_drift() {
        let hunts = std::env::temp_dir().join(format!(
            "rms-hunt-resume-{}-{}",
            std::process::id(),
            now_ms()
        ));
        let run = hunts.join("run-1");
        fs::create_dir_all(&run).unwrap();
        let mut tools = BTreeMap::new();
        tools.insert("rms".to_string(), "sha256:original".to_string());
        let report = HuntReport {
            spec: REPORT_SPEC.to_string(),
            run_id: "run-1".to_string(),
            result: "inconclusive".to_string(),
            source: HuntSource {
                revision: "revision".to_string(),
                declaration_digest: "declarations".to_string(),
                root: ".".to_string(),
            },
            configuration: HuntConfiguration {
                budget_seconds: 10,
                seed: 7,
                jobs: 1,
                module: None,
                assembly: None,
                output: None,
                tools: tools.clone(),
            },
            lanes: Vec::new(),
            findings: Vec::new(),
            proof_scope: empty_scope(),
            elapsed_ms: 0,
            started_at_unix_ms: 0,
            finished_at_unix_ms: None,
        };
        write_report(&run.join("checkpoint.yaml"), &report).unwrap();

        let (_, loaded) = load_resume_report(&hunts, Some("run-1"))
            .unwrap()
            .expect("resume report");
        assert!(validate_resume(&loaded, Some(7), "revision", "declarations").is_ok());
        assert!(validate_resume(&loaded, None, "changed", "declarations")
            .unwrap_err()
            .to_string()
            .contains("source or declaration drift"));
        assert!(validate_resume(&loaded, None, "revision", "changed")
            .unwrap_err()
            .to_string()
            .contains("source or declaration drift"));
        assert!(
            validate_resume(&loaded, Some(8), "revision", "declarations")
                .unwrap_err()
                .to_string()
                .contains("seed drift")
        );
        tools.insert("rms".to_string(), "sha256:changed".to_string());
        assert!(ensure_resume_tool_identities(&report, &tools)
            .unwrap_err()
            .to_string()
            .contains("tool identity drift"));
        fs::remove_dir_all(hunts).unwrap();
    }

    #[test]
    fn interrupted_resume_preserves_partial_artifacts_and_explicitly_reruns_lane() {
        let run_root = std::env::temp_dir().join(format!(
            "rms-hunt-interrupted-{}-{}",
            std::process::id(),
            now_ms()
        ));
        let lane = lane_report(
            "module:guided-1".to_string(),
            "module",
            Some("property".to_string()),
            "probe-exploration",
            "guided-semantic-novelty-v1",
            2,
            "internal".to_string(),
            None,
        );
        let mut running = lane;
        running.status = "running".to_string();
        let segment = sanitize_segment(&running.id);
        fs::create_dir_all(run_root.join("lanes")).unwrap();
        fs::create_dir_all(run_root.join("counterexamples").join(&segment)).unwrap();
        fs::write(
            run_root.join("lanes").join(format!("{segment}.yaml")),
            "spec: rms/hunt-lane-result/v0.1\nstatus: finding\n",
        )
        .unwrap();
        fs::write(
            run_root
                .join("counterexamples")
                .join(&segment)
                .join("partial.yaml"),
            "partial: true\n",
        )
        .unwrap();
        let mut report = HuntReport {
            spec: REPORT_SPEC.to_string(),
            run_id: "interrupted".to_string(),
            result: "inconclusive".to_string(),
            source: HuntSource {
                revision: "revision".to_string(),
                declaration_digest: "declarations".to_string(),
                root: ".".to_string(),
            },
            configuration: HuntConfiguration {
                budget_seconds: 2,
                seed: 7,
                jobs: 1,
                module: None,
                assembly: None,
                output: None,
                tools: BTreeMap::new(),
            },
            lanes: vec![running],
            findings: vec![HuntFinding::raw(
                "module:guided-1".to_string(),
                "behavioral-counterexample".to_string(),
                "partial".to_string(),
                None,
                None,
            )],
            proof_scope: empty_scope(),
            elapsed_ms: 1_250,
            started_at_unix_ms: 0,
            finished_at_unix_ms: None,
        };

        normalize_interrupted_resume(&mut report, &run_root).unwrap();

        assert_eq!(resume_elapsed_ms(&report), 1_250);
        assert_eq!(report.lanes[0].status, "planned");
        assert!(report.lanes[0]
            .diagnostic
            .as_deref()
            .is_some_and(|value| value.contains("explicit rerun")));
        assert!(report.findings.is_empty());
        assert!(!run_root
            .join("lanes")
            .join(format!("{segment}.yaml"))
            .exists());
        assert!(!run_root.join("counterexamples").join(&segment).exists());
        let preserved = WalkDir::new(run_root.join("interrupted"))
            .into_iter()
            .filter_map(std::result::Result::ok)
            .filter(|entry| entry.file_type().is_file())
            .map(|entry| entry.file_name().to_string_lossy().to_string())
            .collect::<Vec<_>>();
        assert!(preserved.contains(&format!("{segment}.yaml")));
        assert!(preserved.contains(&"partial.yaml".to_string()));
        fs::remove_dir_all(run_root).unwrap();
    }

    #[test]
    fn guided_hunt_deadline_bounds_slow_probe_and_kills_its_process_group() {
        let repository = Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(3)
            .expect("repository root");
        let source = repository.join("examples/probe-topologies/source");
        let root = std::env::temp_dir().join(format!(
            "rms-hunt-deadline-{}-{}",
            std::process::id(),
            now_ms()
        ));
        fs::create_dir_all(&root).unwrap();
        fs::copy(
            source.join("machine_probe.fixture"),
            root.join("machine_probe.fixture"),
        )
        .unwrap();
        let marker = root.join("orphan-marker");
        let wrapper = format!(
            r#"if grep -q '"operation": "describe"' "$RMS_PROBE_REQUEST"; then
  exec python machine_probe.fixture
fi
(sleep 3; printf survived > {}) &
sleep 10
"#,
            shell_quote(&marker.display().to_string())
        );
        fs::write(root.join("probe-wrapper.sh"), wrapper).unwrap();
        let manifest = fs::read_to_string(source.join("implementation.fixture"))
            .unwrap()
            .replace("python machine_probe.fixture", "sh probe-wrapper.sh");
        fs::write(root.join("implementation.yaml"), manifest).unwrap();
        let assembly = root.join("assembly.yaml");
        fs::write(
            &assembly,
            serde_yaml::to_string(&serde_json::json!({
                "spec": "rms/probe-assembly/v0.2",
                "instances": [{"id":"source","implementation":"implementation.yaml"}],
                "stimuli": [{
                    "id":"start",
                    "target":"source",
                    "input":{"kind":"command","name":"Start","data":{}}
                }],
                "substitutes": [{
                    "id":"work-substitute",
                    "source":"source",
                    "output":{"kind":"effect","name":"Work"},
                    "outcomes":[{
                        "target":"source",
                        "input":{"kind":"effect-result","name":"Done","data":{}}
                    }]
                }],
                "checks": [{
                    "id":"ready",
                    "when":"quiescent",
                    "assert":{"kind":"state","instance":"source","equals":{"name":"Ready","data":{}}}
                }]
            }))
            .unwrap(),
        )
        .unwrap();
        let run_root = root.join("run");
        prepare_run_storage(&run_root).unwrap();
        let lane = HuntLane {
            report: lane_report(
                "deadline:guided".to_string(),
                "deadline",
                Some("bounded".to_string()),
                "probe-exploration",
                "guided-semantic-novelty-v1",
                2,
                "internal".to_string(),
                None,
            ),
            execution: LaneExecution::GuidedProbe {
                root: root.clone(),
                source_root: root.clone(),
                assembly,
                max_steps: None,
                max_schedules: None,
                max_states: None,
            },
        };
        let started = Instant::now();
        let (report, findings) = execute_lane(
            &lane,
            "deadline-run",
            17,
            &run_root,
            Instant::now() + Duration::from_secs(2),
            Arc::new(AtomicBool::new(false)),
        )
        .unwrap();
        let elapsed = started.elapsed();

        assert_eq!(report.status, "inconclusive");
        assert!(findings.is_empty());
        assert!(elapsed < Duration::from_secs(4), "elapsed: {elapsed:?}");
        std::thread::sleep(Duration::from_millis(1_500));
        assert!(
            !marker.exists(),
            "timed-out probe left a descendant process alive"
        );
        fs::remove_dir_all(root).unwrap();
    }
}
