use serde_json::Value;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn add_module_preview_preserves_project_and_apply_matches_manifest() {
    let unique = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    let root = std::env::temp_dir().join(format!("rms-cli-module-preview-{unique}"));
    fs::create_dir_all(&root).unwrap();
    let run = |args: &[String]| Command::new(env!("CARGO_BIN_EXE_rms"))
        .current_dir(&root).args(args).output().unwrap();
    let init = run(&["init".into(), ".".into(), "--name".into(), "preview-test".into(),
        "--purpose".into(), "Test scaffold preview.".into()]);
    assert!(init.status.success(), "{}", String::from_utf8_lossy(&init.stderr));
    for args in [vec!["init", "-q"], vec!["config", "user.name", "RMS Test"],
        vec!["config", "user.email", "rms@example.test"], vec!["add", "-A"],
        vec!["commit", "-qm", "bootstrap"]] {
        assert!(Command::new("git").current_dir(&root).args(args).status().unwrap().success());
    }
    let intent = serde_json::json!({
        "spec": "rms/intent-model/v0.1", "operation": "design", "change_scope": "new-module",
        "subjects": ["receiver"],
        "facts": {
            "domain_decisions": {"disposition":"required","basis":"explicit","source_quote":"receiver decisions"},
            "lifecycle": {"disposition":"required","basis":"explicit","source_quote":"receiver lifecycle"},
            "effects": {"disposition":"absent","basis":"explicit","source_quote":"pure decisions"},
            "runnable_surface": {"disposition":"absent","basis":"explicit","source_quote":"library only"},
            "reuse": {"disposition":"required","basis":"explicit","source_quote":"reusable receiver"}
        },
        "responsibilities": [
            {"id":"receiver-lifecycle","kind":"workflow","summary":"Own receiver lifecycle."},
            {"id":"receiver-decisions","kind":"decision","summary":"Own receiver decisions."}
        ],
        "surface_kinds": [], "binding_preferences": ["rust"], "open_questions": []
    });
    let design = run(&["design".into(), "--task".into(), "Create reusable receiver decisions and receiver lifecycle; pure decisions, library only.".into(),
        "--intent-json".into(), intent.to_string(), "--json".into()]);
    assert!(design.status.success(), "{} {}", String::from_utf8_lossy(&design.stdout), String::from_utf8_lossy(&design.stderr));
    let design: Value = serde_json::from_slice(&design.stdout).unwrap();
    assert_eq!(design["result"], "ready", "{design}");
    let mut args = design["decision"]["scaffold"]["args"].as_array().unwrap().iter()
        .map(|value| value.as_str().unwrap().to_string()).collect::<Vec<_>>();
    assert_eq!(args[0], "add-module");
    let target = root.join(&args[1]);
    args.extend(["--route-receipt".into(), design["run_id"].as_str().unwrap().into()]);
    let snapshot = || walkdir::WalkDir::new(&root).into_iter().map(|entry| {
        let entry = entry.unwrap();
        let bytes = entry.file_type().is_file().then(|| fs::read(entry.path()).unwrap());
        (entry.path().strip_prefix(&root).unwrap().to_path_buf(), bytes)
    }).collect::<std::collections::BTreeMap<_, _>>();
    let before = snapshot();
    let mut preview = args.clone();
    preview.push("--dry-run".into());
    let result = run(&preview);
    assert!(result.status.success(), "{}", String::from_utf8_lossy(&result.stderr));
    assert_eq!(snapshot(), before);
    assert!(!target.exists());
    let output = String::from_utf8(result.stdout).unwrap();
    assert!(output.contains("module.yaml") && output.contains("Cargo.toml"));
    let intended = output.lines().filter_map(|line| line.strip_prefix("create "))
        .map(|path| root.join(path)).collect::<std::collections::BTreeSet<_>>();

    // A different scaffold and a missing receipt must fail without side effects.
    let mut invalid = preview.clone();
    let name = invalid.iter().position(|arg| arg == "--name").unwrap() + 1;
    invalid[name] = "wrong-owner".into();
    assert!(!run(&invalid).status.success());
    assert_eq!(snapshot(), before);
    let mut invalid = preview.clone();
    let receipt = invalid.iter().position(|arg| arg == "--route-receipt").unwrap() + 1;
    invalid[receipt] = "missing-receipt".into();
    assert!(!run(&invalid).status.success());
    assert_eq!(snapshot(), before);
    let mut invalid = preview.clone();
    invalid.push("--record".into());
    assert!(!run(&invalid).status.success());
    assert_eq!(snapshot(), before);

    let applied = run(&args);
    assert!(applied.status.success(), "{}", String::from_utf8_lossy(&applied.stderr));
    let actual = walkdir::WalkDir::new(&target).into_iter().map(Result::unwrap)
        .filter(|entry| entry.file_type().is_file()).map(|entry| entry.path().to_path_buf())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(actual, intended);
    let after = snapshot();
    assert!(!run(&preview).status.success());
    assert_eq!(snapshot(), after);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn cli_version_flag_uses_package_version() {
    let output = Command::new(env!("CARGO_BIN_EXE_rms"))
        .arg("--version")
        .output()
        .expect("run the RMS CLI");

    assert!(
        output.status.success(),
        "rms --version failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let version = String::from_utf8(output.stdout).expect("UTF-8 version output");
    assert!(version.starts_with(&format!("rms {} (revision ", env!("CARGO_PKG_VERSION"))));
    assert!(version.ends_with(")\n"));
}

#[test]
fn leaf_retirement_cli_preserves_native_code_and_checks_both_deltas() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("rms-cli-retirement-{unique}"));
    fs::create_dir_all(root.join("modules/unused/src")).unwrap();
    fs::write(root.join("modules/unused/module.yaml"), "spec: rms/module/v0.1\nmodule: {name: unused, version: 0.1.0, kind: adapter, purpose: Unused scaffold}\nowns: {}\nprovides: {}\nrequires: {}\neffects: []\n").unwrap();
    fs::write(
        root.join("modules/unused/src/history.txt"),
        "historical bytes\n",
    )
    .unwrap();
    fs::write(root.join("native.js"), "export const preserved = true;\n").unwrap();
    fs::write(root.join(".gitignore"), ".rms/runs/\n.rms/cache/\n").unwrap();
    let git = |args: &[&str]| {
        let output = Command::new("git")
            .current_dir(&root)
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    };
    git(&["init", "-q"]);
    git(&["config", "user.name", "Retirement CLI Test"]);
    git(&["config", "user.email", "retirement@example.invalid"]);
    git(&["add", "-A"]);
    git(&["commit", "-qm", "baseline"]);
    let run = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_rms"))
            .current_dir(&root)
            .args(args)
            .output()
            .unwrap()
    };
    let plan = run(&[
        "retire-module",
        "plan",
        "modules/unused/module.yaml",
        "--root",
        ".",
        "--reason",
        "Retire unused scaffold and preserve native code",
    ]);
    assert!(
        plan.status.success(),
        "{}",
        String::from_utf8_lossy(&plan.stderr)
    );
    let plan: Value = serde_json::from_slice(&plan.stdout).unwrap();
    let receipt = plan["route_receipt"].as_str().unwrap();
    let dry = run(&[
        "retire-module",
        "apply",
        "modules/unused/module.yaml",
        "--root",
        ".",
        "--route-receipt",
        receipt,
        "--dry-run",
    ]);
    assert!(
        dry.status.success(),
        "{}",
        String::from_utf8_lossy(&dry.stderr)
    );
    assert!(root.join("modules/unused/module.yaml").exists());
    assert!(!root.join(".rms/retirements").exists());
    let applied = run(&[
        "retire-module",
        "apply",
        "modules/unused/module.yaml",
        "--root",
        ".",
        "--route-receipt",
        receipt,
    ]);
    assert!(
        applied.status.success(),
        "{}",
        String::from_utf8_lossy(&applied.stderr)
    );
    let applied: Value = serde_json::from_slice(&applied.stdout).unwrap();
    let record = PathBuf::from(applied["record"].as_str().unwrap());
    assert!(!root.join("modules/unused").exists());
    assert_eq!(
        fs::read(root.join("native.js")).unwrap(),
        b"export const preserved = true;\n"
    );
    for mode in ["--changes", "--committed"] {
        if mode == "--committed" {
            git(&["add", "-A"]);
            git(&["commit", "-qm", "retire leaf"]);
        }
        let check = run(&["check", mode, "--root", ".", "--json", "--details"]);
        assert!(
            check.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&check.stdout),
            String::from_utf8_lossy(&check.stderr)
        );
        assert!(String::from_utf8_lossy(&check.stdout).contains("module-retirements"));
    }
    let check = run(&["retire-module", "check", "--root", "."]);
    assert!(check.status.success());
    fs::write(
        record.parent().unwrap().join("archive/src/history.txt"),
        "tampered",
    )
    .unwrap();
    assert!(!run(&["retire-module", "check", "--root", "."])
        .status
        .success());
    assert!(!run(&["check", "--changes", "--root", "."]).status.success());
    fs::remove_dir_all(&root).unwrap();
}

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .expect("repository root")
        .to_path_buf()
}

#[test]
fn ci_workflows_fetch_complete_provenance_history() {
    for file in [".github/workflows/ci.yml", ".github/workflows/release.yml"] {
        let workflow: serde_yaml::Value =
            serde_yaml::from_str(&fs::read_to_string(repository_root().join(file)).unwrap())
                .unwrap();
        let mut checkouts = 0;
        for job in workflow["jobs"].as_mapping().unwrap().values() {
            for step in job["steps"].as_sequence().unwrap() {
                if step["uses"]
                    .as_str()
                    .is_some_and(|value| value.starts_with("actions/checkout@"))
                {
                    checkouts += 1;
                    assert_eq!(
                        step["with"]["fetch-depth"].as_u64(),
                        Some(0),
                        "{file}: retirement provenance requires full Git history"
                    );
                }
            }
        }
        assert!(checkouts > 0, "{file}: no checkout found");
    }
}

fn run_probe(arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_rms"))
        .current_dir(repository_root())
        .arg("probe")
        .args(arguments)
        .output()
        .expect("run RMS probe")
}

#[test]
fn probe_assembly_describe_and_five_instance_execution_are_stable() {
    let description = run_probe(&[
        "--file",
        "examples/probes/series.yaml",
        "--describe",
        "--json",
    ]);
    assert!(
        description.status.success(),
        "{}",
        String::from_utf8_lossy(&description.stderr)
    );
    let description: Value =
        serde_json::from_slice(&description.stdout).expect("assembly description JSON");
    assert_eq!(description["result"], "ready");
    assert_eq!(description["instances"].as_array().map(Vec::len), Some(2));

    let execution = run_probe(&["--file", "examples/probes/five-modules.yaml", "--json"]);
    assert!(
        execution.status.success(),
        "{}",
        String::from_utf8_lossy(&execution.stderr)
    );
    let trace: Value = serde_json::from_slice(&execution.stdout).expect("system trace JSON");
    assert_eq!(trace["spec"], "rms/probe-system-trace/v0.1");
    assert_eq!(trace["result"], "pass");
    assert_eq!(trace["instances"].as_array().map(Vec::len), Some(5));
}

#[test]
fn probe_counterexample_exit_codes_distinguish_reproduced_and_invalid() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock")
        .as_nanos();
    let counterexample =
        std::env::temp_dir().join(format!("rms-probe-counterexample-{unique}.json"));
    let counterexample_arg = counterexample.to_string_lossy().to_string();

    let failure = run_probe(&[
        "--file",
        "examples/probes/repeated-rust-failure.yaml",
        "--explore",
        "--out",
        &counterexample_arg,
        "--json",
    ]);
    assert_eq!(failure.status.code(), Some(1));
    let artifact: Value =
        serde_json::from_slice(&fs::read(&counterexample).expect("counterexample artifact"))
            .expect("counterexample JSON");
    assert_eq!(artifact["spec"], "rms/probe-counterexample/v0.1");

    let replay = run_probe(&["--replay", &counterexample_arg, "--json"]);
    assert_eq!(replay.status.code(), Some(1));
    let replay: Value = serde_json::from_slice(&replay.stdout).expect("replay report JSON");
    assert_eq!(replay["result"], "reproduced");

    let human_replay = run_probe(&["--replay", &counterexample_arg]);
    assert_eq!(human_replay.status.code(), Some(1));
    let human_replay = String::from_utf8(human_replay.stdout).expect("human replay UTF-8");
    assert!(human_replay.starts_with("RMS probe replay: reproduced\ncheck: "));
    assert!(human_replay.contains("first bad transition: "));
    assert!(human_replay.contains("exit: 1 (the recorded failure reproduced)"));
    assert!(human_replay.contains("full trace: "));

    let invalid = run_probe(&["--replay", "examples/probes/series.yaml", "--json"]);
    assert_eq!(invalid.status.code(), Some(2));
    let invalid: Value = serde_json::from_slice(&invalid.stdout).expect("invalid replay JSON");
    assert_eq!(invalid["result"], "invalid");

    let _ = fs::remove_file(counterexample);
}

#[test]
fn probe_assembly_from_stdin_resolves_paths_from_the_working_directory() {
    let source = fs::read_to_string(repository_root().join("examples/probes/series.yaml"))
        .expect("series assembly")
        .replace("../probe-topologies/", "examples/probe-topologies/");
    let mut child = Command::new(env!("CARGO_BIN_EXE_rms"))
        .current_dir(repository_root())
        .args(["probe", "--file", "-", "--describe", "--json"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn stdin assembly probe");
    child
        .stdin
        .take()
        .expect("probe stdin")
        .write_all(source.as_bytes())
        .expect("write probe assembly");
    let output = child.wait_with_output().expect("wait for stdin probe");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let description: Value =
        serde_json::from_slice(&output.stdout).expect("stdin assembly description");
    assert_eq!(description["result"], "ready");
    assert_eq!(description["instances"].as_array().map(Vec::len), Some(2));
}

#[test]
fn probe_without_out_writes_no_artifacts() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock")
        .as_nanos();
    let working_directory = std::env::temp_dir().join(format!("rms-probe-no-output-{unique}"));
    fs::create_dir(&working_directory).expect("create isolated probe working directory");
    let assembly = repository_root().join("examples/probes/series.yaml");

    let output = Command::new(env!("CARGO_BIN_EXE_rms"))
        .current_dir(&working_directory)
        .args(["probe", "--file"])
        .arg(assembly)
        .arg("--json")
        .output()
        .expect("run RMS probe without --out");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        fs::read_dir(&working_directory)
            .expect("inspect isolated probe working directory")
            .count(),
        0,
        "probe wrote an artifact without --out"
    );

    fs::remove_dir(&working_directory).expect("remove isolated probe working directory");
}

#[test]
fn hunt_runs_nightly_lane_in_an_isolated_checkout_and_resumes() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("rms-hunt-cli-{unique}"));
    fs::create_dir_all(&root).expect("create hunt fixture");
    fs::write(root.join(".gitignore"), ".rms/\n").expect("write ignore file");
    fs::write(
        root.join("implementation.yaml"),
        r#"spec: rms/implementation/v0.1
module: hunt-fixture
binding: executable
source:
  root: .
  public_entrypoint: runner.sh
commands:
  nightly: sh runner.sh
architecture:
  shape: domain-engine
  reliability:
    properties:
      - id: overnight-oracle
        proves: overnight-law
        kind: property
        input_space: generated cases
        operation: exercise the fixture
        oracle: [the runner completes]
        evidence: { path: evidence.md }
        counterexamples: { path: counterexamples }
        realizations:
          - profile: nightly
            strategy: mutation-tester
            command: nightly
            runner: runner.sh#run
          - profile: ci
            strategy: static-analyzer
            command: nightly
            runner: runner.sh#run
"#,
    )
    .expect("write implementation");
    fs::write(root.join("evidence.md"), "Fixture evidence.\n").expect("write evidence");
    fs::create_dir(root.join("counterexamples")).expect("create counterexample directory");
    fs::write(
        root.join("runner.sh"),
        r#"#!/bin/sh
set -eu
test -n "${RMS_HUNT_RUN_ID:-}"
test -n "${RMS_HUNT_SEED:-}"
test -n "${RMS_HUNT_BUDGET_SECONDS:-}"
test -n "${RMS_HUNT_OUTPUT:-}"
printf '%s\n' \
  'spec: rms/hunt-lane-result/v0.1' \
  'status: pass' \
  'metrics:' \
  '  mutants: 1' > "$RMS_HUNT_OUTPUT"
"#,
    )
    .expect("write runner");
    for arguments in [
        ["init"].as_slice(),
        ["config", "user.email", "rms@example.test"].as_slice(),
        ["config", "user.name", "RMS Test"].as_slice(),
        ["add", "."].as_slice(),
        ["commit", "-m", "baseline"].as_slice(),
    ] {
        let output = Command::new("git")
            .current_dir(&root)
            .args(arguments)
            .output()
            .expect("prepare hunt git fixture");
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let selected_plan = Command::new(env!("CARGO_BIN_EXE_rms"))
        .current_dir(&root)
        .args([
            "hunt",
            "--root",
            ".",
            "--budget",
            "10s",
            "--seed",
            "17",
            "--profile",
            "nightly",
            "--lane",
            "mutation",
            "--dry-run",
            "--json",
        ])
        .output()
        .expect("plan selected hunt lanes");
    assert!(
        selected_plan.status.success(),
        "{}",
        String::from_utf8_lossy(&selected_plan.stderr)
    );
    let selected_plan: Value =
        serde_json::from_slice(&selected_plan.stdout).expect("selected hunt plan JSON");
    assert_eq!(selected_plan["configuration"]["profiles"][0], "nightly");
    assert_eq!(selected_plan["configuration"]["lanes"][0], "mutation");
    assert_eq!(selected_plan["lanes"].as_array().map(Vec::len), Some(1));
    assert_eq!(
        selected_plan["exclusions"].as_array().map(Vec::len),
        Some(1)
    );

    let first = Command::new(env!("CARGO_BIN_EXE_rms"))
        .current_dir(&root)
        .args([
            "hunt",
            "--root",
            ".",
            "--budget",
            "10s",
            "--seed",
            "17",
            "--jobs",
            "2",
            "--out",
            ".rms/export.json",
            "--json",
        ])
        .output()
        .expect("run hunt");
    assert!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    let report: Value = serde_json::from_slice(&first.stdout).expect("hunt report JSON");
    assert_eq!(report["spec"], "rms/hunt-report/v0.2");
    assert_eq!(report["result"], "clean-under-recorded-bounds");
    assert_eq!(report["configuration"]["seed"], 17);
    assert_eq!(report["configuration"]["budget_seconds"], 10);
    assert_eq!(report["configuration"]["jobs"], 2);
    assert!(report["configuration"]["output"]
        .as_str()
        .is_some_and(|path| path.ends_with("/.rms/export.json")));
    assert_eq!(report["lanes"][0]["status"], "pass");
    assert_eq!(report["lanes"][0]["metrics"]["mutants"], 1);
    let exported: Value = serde_json::from_slice(
        &fs::read(root.join(".rms/export.json")).expect("read exported JSON report"),
    )
    .expect("exported report is JSON");
    assert_eq!(exported["run_id"], report["run_id"]);

    let resumed = Command::new(env!("CARGO_BIN_EXE_rms"))
        .current_dir(&root)
        .args(["hunt", "--root", ".", "--resume", "latest", "--json"])
        .output()
        .expect("resume hunt");
    assert!(
        resumed.status.success(),
        "{}",
        String::from_utf8_lossy(&resumed.stderr)
    );
    let resumed_report: Value =
        serde_json::from_slice(&resumed.stdout).expect("resumed hunt report JSON");
    assert_eq!(resumed_report["run_id"], report["run_id"]);
    assert_eq!(resumed_report["result"], "clean-under-recorded-bounds");
    assert_eq!(resumed_report["configuration"], report["configuration"]);
    assert_eq!(
        resumed_report["finished_at_unix_ms"], report["finished_at_unix_ms"],
        "resuming a finalized run must not rewrite its provenance timestamp"
    );
    let checkpoint: Value = serde_yaml::from_slice(
        &fs::read(
            root.join(".rms/hunts")
                .join(report["run_id"].as_str().expect("run id"))
                .join("checkpoint.yaml"),
        )
        .expect("read finalized checkpoint"),
    )
    .expect("finalized checkpoint YAML");
    assert_eq!(checkpoint["result"], "clean-under-recorded-bounds");
    assert!(checkpoint["finished_at_unix_ms"].is_number());
    let drifted = Command::new(env!("CARGO_BIN_EXE_rms"))
        .current_dir(&root)
        .args([
            "hunt", "--root", ".", "--resume", "latest", "--budget", "11s",
        ])
        .output()
        .expect("reject changed resume configuration");
    assert!(!drifted.status.success());
    assert!(String::from_utf8_lossy(&drifted.stderr).contains("budget configuration drift"));
    assert!(!root.join("checkout").exists());
    assert!(
        Command::new("git")
            .current_dir(&root)
            .args(["status", "--porcelain", "--untracked-files=normal"])
            .output()
            .expect("inspect hunt fixture")
            .stdout
            .is_empty(),
        "hunt mutated the committed source checkout"
    );

    fs::remove_dir_all(root).expect("remove hunt fixture");
}

#[test]
fn concurrent_multi_module_hunt_dry_runs_do_not_race_git_worktrees() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("rms-hunt-concurrent-cli-{unique}"));
    fs::create_dir_all(&root).expect("create hunt fixture");
    fs::write(root.join(".gitignore"), ".rms/\n").expect("write ignore file");
    for index in 0..4 {
        let module = root.join(format!("modules/module-{index}"));
        fs::create_dir_all(&module).expect("create module");
        fs::write(
            module.join("module.yaml"),
            format!(
                "spec: rms/module/v0.1\nmodule: {{name: module-{index}}}\npurpose: Hunt fixture.\n"
            ),
        )
        .expect("write module");
        fs::write(
            module.join("implementation.yaml"),
            format!(
                "spec: rms/implementation/v0.1\nmodule: module-{index}\nbinding: executable\narchitecture:\n  reliability:\n    properties: []\n    fuzz_targets: []\n"
            ),
        )
        .expect("write implementation");
    }
    for arguments in [
        ["init"].as_slice(),
        ["config", "user.email", "rms@example.test"].as_slice(),
        ["config", "user.name", "RMS Test"].as_slice(),
        ["add", "."].as_slice(),
        ["commit", "-m", "baseline"].as_slice(),
    ] {
        let output = Command::new("git")
            .current_dir(&root)
            .args(arguments)
            .output()
            .expect("prepare hunt git fixture");
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let handles = (0..4)
        .map(|index| {
            let root = root.clone();
            std::thread::spawn(move || {
                Command::new(env!("CARGO_BIN_EXE_rms"))
                    .current_dir(&root)
                    .args([
                        "hunt",
                        "--root",
                        ".",
                        "--module",
                        &format!("modules/module-{index}/module.yaml"),
                        "--budget",
                        "2s",
                        "--dry-run",
                        "--json",
                    ])
                    .output()
                    .expect("run concurrent hunt dry-run")
            })
        })
        .collect::<Vec<_>>();
    for handle in handles {
        let output = handle.join().expect("join hunt dry-run");
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let report: Value = serde_json::from_slice(&output.stdout).expect("hunt JSON");
        assert_eq!(report["spec"], "rms/hunt-report/v0.2");
    }
    let runs = fs::read_dir(root.join(".rms/hunts"))
        .expect("read hunt runs")
        .filter_map(Result::ok)
        .filter(|entry| entry.path().is_dir())
        .count();
    assert_eq!(runs, 4);
    let worktrees = Command::new("git")
        .current_dir(&root)
        .args(["worktree", "list", "--porcelain"])
        .output()
        .expect("list worktrees");
    assert_eq!(
        String::from_utf8_lossy(&worktrees.stdout)
            .matches("worktree ")
            .count(),
        1
    );
    fs::remove_dir_all(root).expect("remove hunt fixture");
}

#[test]
fn check_changes_reports_outside_coverage_without_broad_verification() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("rms-check-outside-cli-{unique}"));
    fs::create_dir_all(root.join("native")).expect("create outside-coverage fixture");
    fs::write(root.join(".gitignore"), ".rms/cache/\n").expect("write ignore file");
    fs::write(root.join("native/client.txt"), "baseline\n").expect("write baseline file");
    for arguments in [
        ["init"].as_slice(),
        ["config", "user.email", "rms@example.test"].as_slice(),
        ["config", "user.name", "RMS Test"].as_slice(),
        ["add", "."].as_slice(),
        ["commit", "-m", "baseline"].as_slice(),
    ] {
        let output = Command::new("git")
            .current_dir(&root)
            .args(arguments)
            .output()
            .expect("prepare check git fixture");
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    fs::write(root.join("native/client.txt"), "candidate\n").expect("write candidate file");

    let output = Command::new(env!("CARGO_BIN_EXE_rms"))
        .current_dir(&root)
        .args(["check", "--changes", "--root", ".", "--json", "--details"])
        .output()
        .expect("run affected check");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value = serde_json::from_slice(&output.stdout).expect("affected check JSON");
    assert_eq!(report["schema"], "rms.surface/v2");
    assert_eq!(report["mode"], "changes");
    assert_eq!(report["result"], "pass");
    assert_eq!(
        report["coverage"]["closures"].as_array().map(Vec::len),
        Some(0)
    );
    assert_eq!(report["delta"]["coverage_status"], "partial");
    assert_eq!(
        report["delta"]["outside_coverage_changed_paths"][0],
        "native/client.txt"
    );
    assert!(report["coverage"]["certification"]
        .as_str()
        .is_some_and(|value| value.contains("not certified by RMS")));

    for arguments in [
        ["add", "native/client.txt"].as_slice(),
        ["commit", "-m", "outside candidate"].as_slice(),
    ] {
        let output = Command::new("git")
            .current_dir(&root)
            .args(arguments)
            .output()
            .expect("commit outside-coverage candidate");
        assert!(output.status.success());
    }
    let exhaustive = Command::new(env!("CARGO_BIN_EXE_rms"))
        .current_dir(&root)
        .args(["check", "--all", "--root", ".", "--json", "--details"])
        .output()
        .expect("run exhaustive check");
    assert!(!exhaustive.status.success());
    let exhaustive: Value =
        serde_json::from_slice(&exhaustive.stdout).expect("exhaustive check JSON");
    assert_eq!(exhaustive["mode"], "all");
    assert_eq!(exhaustive["result"], "fail");
    assert!(exhaustive["next_action"]["args"]
        .as_array()
        .is_some_and(|args| args.iter().any(|arg| arg == "--all")));
    assert!(exhaustive["details"]["audit"]["checks"]
        .as_array()
        .is_some_and(|checks| checks
            .iter()
            .any(|check| { check["id"] == "modules.discovered" && check["result"] == "fail" })));

    fs::remove_dir_all(root).expect("remove outside-coverage fixture");
}
