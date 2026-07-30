use super::{execute_proof_command, get_path, get_str, sha256_bytes};
use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use serde_yaml::Value as YamlValue;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use walkdir::WalkDir;

const REPORT_SPEC: &str = "rms/hunt-report/v0.1";
const LANE_RESULT_SPEC: &str = "rms/hunt-lane-result/v0.1";

pub(super) struct HuntRequest {
    root: PathBuf,
    module: Option<PathBuf>,
    budget: String,
    seed: Option<u64>,
    jobs: usize,
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
        budget: String,
        seed: Option<u64>,
        jobs: usize,
        resume: Option<String>,
        out: Option<PathBuf>,
        dry_run: bool,
        json: bool,
    ) -> Self {
        Self {
            root,
            module,
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
    lane: String,
    kind: String,
    summary: String,
    artifact: Option<String>,
    replay: Option<String>,
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
    if request.jobs == 0 {
        bail!("hunt jobs must be at least 1");
    }
    let jobs = request.jobs.min(
        std::thread::available_parallelism()
            .map(usize::from)
            .unwrap_or(1),
    );
    let root = request.root.canonicalize().with_context(|| {
        format!(
            "hunt root `{}` could not be resolved",
            request.root.display()
        )
    })?;
    let budget_seconds = parse_duration_seconds(&request.budget)?;
    let revision = git_output(&root, &["rev-parse", "HEAD"])?;
    ensure_clean_commit(&root)?;
    let declaration_digest = declaration_digest(&root, request.module.as_deref())?;
    let hunts_root = root.join(".rms/hunts");
    fs::create_dir_all(&hunts_root)?;
    let (run_id, seed, mut prior) = resolve_run(
        &hunts_root,
        request.resume.as_deref(),
        request.seed,
        &revision,
        &declaration_digest,
    )?;
    let run_root = hunts_root.join(&run_id);
    fs::create_dir_all(&run_root)?;
    let started_at_unix_ms = prior
        .as_ref()
        .map(|report| report.started_at_unix_ms)
        .unwrap_or_else(now_ms);
    let worktree = run_root.join("checkout");
    prepare_isolated_checkout(&root, &worktree, &revision)?;
    let relative_module = request
        .module
        .as_ref()
        .map(|path| relative_to_root(&root, path))
        .transpose()?;
    let mut lanes = discover_lanes(
        &worktree,
        relative_module.as_deref(),
        budget_seconds,
        &run_root,
    )?;
    let tools = tool_identities(&mut lanes);
    if let Some(previous) = &prior {
        ensure_resume_tool_identities(previous, &tools)?;
    }
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
            jobs,
            module: relative_module
                .as_ref()
                .map(|path| path.display().to_string()),
            tools,
        },
        lanes: lanes.iter().map(|lane| lane.report.clone()).collect(),
        findings: prior
            .take()
            .map(|report| report.findings)
            .unwrap_or_default(),
        proof_scope: empty_scope(),
        started_at_unix_ms,
        finished_at_unix_ms: None,
    };
    write_report(&run_root.join("checkpoint.yaml"), &report)?;
    if request.dry_run {
        report.result = if lanes.is_empty() {
            "unsupported".to_string()
        } else {
            "inconclusive".to_string()
        };
        report.proof_scope = proof_scope(&report.lanes);
        report.finished_at_unix_ms = Some(now_ms());
        finish_report(&root, &worktree, &run_root, request.out.as_deref(), &report)?;
        return print_report(&report, request.json);
    }

    let hunt_started = Instant::now();
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
        let elapsed = hunt_started.elapsed().as_secs();
        if elapsed >= budget_seconds {
            for index in &pending[cursor..] {
                lanes[*index].report.status = "inconclusive".to_string();
                lanes[*index].report.diagnostic = Some("total hunt budget exhausted".to_string());
            }
            break;
        }
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
        let remaining = budget_seconds.saturating_sub(elapsed).max(1);
        for index in batch {
            lanes[*index].report.status = "running".to_string();
            lanes[*index].report.budget_seconds =
                lanes[*index].report.budget_seconds.min(remaining);
        }
        report.lanes = lanes.iter().map(|lane| lane.report.clone()).collect();
        write_report(&run_root.join("checkpoint.yaml"), &report)?;

        let results = std::thread::scope(|scope| {
            batch
                .iter()
                .map(|index| {
                    let lane = &lanes[*index];
                    let run_id = &run_id;
                    let run_root = &run_root;
                    (
                        *index,
                        scope.spawn(move || {
                            execute_lane(lane, run_id, seed.wrapping_add(*index as u64), run_root)
                        }),
                    )
                })
                .collect::<Vec<_>>()
                .into_iter()
                .map(|(index, handle)| {
                    (
                        index,
                        handle
                            .join()
                            .unwrap_or_else(|_| Err(anyhow!("hunt lane panicked"))),
                    )
                })
                .collect::<Vec<_>>()
        });
        for (index, lane_result) in results {
            match lane_result {
                Ok((lane_report, findings)) => {
                    lanes[index].report = lane_report;
                    report.findings.extend(findings);
                }
                Err(error) => {
                    lanes[index].report.status = "invalid".to_string();
                    lanes[index].report.diagnostic = Some(error.to_string());
                }
            }
        }
        report.lanes = lanes.iter().map(|lane| lane.report.clone()).collect();
        write_report(&run_root.join("checkpoint.yaml"), &report)?;
        cursor = batch_end;
    }
    report.lanes = lanes.into_iter().map(|lane| lane.report).collect();
    report.result = hunt_result(&report);
    report.proof_scope = proof_scope(&report.lanes);
    report.finished_at_unix_ms = Some(now_ms());
    finish_report(&root, &worktree, &run_root, request.out.as_deref(), &report)?;
    print_report(&report, request.json)?;
    if matches!(report.result.as_str(), "invalid" | "unsupported") {
        bail!("RMS hunt {}", report.result);
    }
    Ok(())
}

fn discover_lanes(
    checkout: &Path,
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
            .filter(|property| get_path(property, &["temporal"]).is_some())
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
        let mut relationship_assemblies = BTreeMap::<PathBuf, YamlValue>::new();
        for property in &properties {
            let property_id = get_str(property, &["id"]).unwrap_or("unnamed").to_string();
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
                relationship_assemblies.insert(
                    implementation.parent().unwrap_or(checkout).join(assembly),
                    exploration.clone(),
                );
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
                Some("rms/property-analysis/v0.1") => (
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
        for (analysis_index, (assembly, exploration)) in
            relationship_assemblies.into_iter().enumerate()
        {
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
) -> Result<(HuntLaneReport, Vec<HuntFinding>)> {
    let mut report = lane.report.clone();
    let started = Instant::now();
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
            execute_proof_command(root, &command, &[], report.budget_seconds)?
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
            report.budget_seconds,
        )?,
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
            findings.push(HuntFinding {
                lane: report.id.clone(),
                kind: "behavioral-counterexample".to_string(),
                summary: if report.kind == "historical-replay" {
                    "a historical counterexample still reproduces".to_string()
                } else {
                    "the smoke baseline found a reproducible behavioral failure".to_string()
                },
                artifact: None,
                replay: Some(report.command.clone()),
            });
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
            findings.push(HuntFinding {
                lane: report.id.clone(),
                kind: "behavioral-counterexample".to_string(),
                summary: "a historical property counterexample still reproduces".to_string(),
                artifact: None,
                replay: Some(report.command.clone()),
            });
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
            let candidate = HuntFinding {
                lane: report.id.clone(),
                kind: get_str(finding, &["kind"]).unwrap_or("invalid").to_string(),
                summary: get_str(finding, &["summary"])
                    .unwrap_or("hunt lane reported a finding")
                    .to_string(),
                artifact: get_str(finding, &["artifact"]).map(ToString::to_string),
                replay: get_str(finding, &["replay"]).map(ToString::to_string),
            };
            if behavioral_finding(&candidate.kind) {
                let Some(replay) = candidate.replay.as_deref() else {
                    report.status = "invalid".to_string();
                    report.diagnostic =
                        Some("behavioral finding omitted a replay command".to_string());
                    continue;
                };
                let replay_output = execute_proof_command(
                    lane_root(lane),
                    replay,
                    &[("RMS_HUNT_RUN_ID", run_id), ("RMS_HUNT_SEED", &seed_string)],
                    report.budget_seconds.min(120),
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
                            let replay = replay_property_analysis(
                                lane_root(lane),
                                Path::new(path),
                                report.budget_seconds.min(120),
                            )?;
                            if replay.status.success() && !replay.timed_out {
                                report.status = "finding".to_string();
                                findings.push(HuntFinding {
                                    lane: report.id.clone(),
                                    kind: "behavioral-counterexample".to_string(),
                                    summary: if report.kind == "trace-evaluation" {
                                        "real-trace evaluation found a replayable property violation"
                                            .to_string()
                                    } else {
                                        "probe exploration found a replayable property violation"
                                            .to_string()
                                    },
                                    artifact: Some(path.clone()),
                                    replay: Some(format!(
                                        "rms property replay {}",
                                        shell_quote(path)
                                    )),
                                });
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
    Ok((report, findings))
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
        LaneExecution::Rms { root, .. } | LaneExecution::Project { root, .. } => root,
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
        println!(
            "lanes: {} ({} finding(s))",
            report.lanes.len(),
            report.findings.len()
        );
        for lane in &report.lanes {
            println!(
                "  - {} [{}] {} ({} ms)",
                lane.id, lane.strategy, lane.status, lane.elapsed_ms
            );
        }
        for finding in &report.findings {
            println!(
                "  finding [{}] {}: {}",
                finding.kind, finding.lane, finding.summary
            );
            if let Some(replay) = &finding.replay {
                println!("    replay: {replay}");
            }
        }
        println!("claim: {}", report.proof_scope.claim);
    }
    Ok(())
}

fn resolve_run(
    hunts_root: &Path,
    resume: Option<&str>,
    requested_seed: Option<u64>,
    revision: &str,
    declaration_digest: &str,
) -> Result<(String, u64, Option<HuntReport>)> {
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
        if report.source.revision != revision
            || report.source.declaration_digest != declaration_digest
        {
            bail!("hunt resume rejected source or declaration drift");
        }
        if requested_seed.is_some_and(|seed| seed != report.configuration.seed) {
            bail!("hunt resume rejected seed drift");
        }
        return Ok((run_id, report.configuration.seed, Some(report)));
    }
    let seed = requested_seed.unwrap_or_else(|| {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64
            ^ std::process::id() as u64
    });
    let run_id = format!(
        "{}-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        &revision[..revision.len().min(12)]
    );
    Ok((run_id, seed, None))
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

fn declaration_digest(root: &Path, selected_module: Option<&Path>) -> Result<String> {
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
        let selected = if module.is_absolute() {
            module.to_path_buf()
        } else {
            root.join(module)
        };
        let selected_root = selected.parent().unwrap_or(root);
        files.retain(|path| path.starts_with(selected_root));
    }
    files.sort();
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

fn relative_to_root(root: &Path, path: &Path) -> Result<PathBuf> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    };
    absolute
        .strip_prefix(root)
        .map(Path::to_path_buf)
        .map_err(|_| anyhow!("module must be inside the hunt root"))
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
        include_str!("../../../../schemas/hunt-report.schema.json"),
        "hunt report",
    )?;
    fs::write(path, serde_yaml::to_string(report)?)?;
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
            started_at_unix_ms: 0,
            finished_at_unix_ms: None,
        };
        report.lanes[0].status = "pass".to_string();
        assert_eq!(hunt_result(&report), "clean-under-recorded-bounds");
        report.findings.push(HuntFinding {
            lane: "lane".to_string(),
            kind: "surviving-mutant".to_string(),
            summary: "oracle missed mutation".to_string(),
            artifact: None,
            replay: None,
        });
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
                tools: tools.clone(),
            },
            lanes: Vec::new(),
            findings: Vec::new(),
            proof_scope: empty_scope(),
            started_at_unix_ms: 0,
            finished_at_unix_ms: None,
        };
        write_report(&run.join("checkpoint.yaml"), &report).unwrap();

        assert!(resolve_run(&hunts, Some("run-1"), Some(7), "revision", "declarations").is_ok());
        assert!(
            resolve_run(&hunts, Some("run-1"), None, "changed", "declarations")
                .unwrap_err()
                .to_string()
                .contains("source or declaration drift")
        );
        assert!(
            resolve_run(&hunts, Some("run-1"), None, "revision", "changed")
                .unwrap_err()
                .to_string()
                .contains("source or declaration drift")
        );
        assert!(
            resolve_run(&hunts, Some("run-1"), Some(8), "revision", "declarations")
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
}
