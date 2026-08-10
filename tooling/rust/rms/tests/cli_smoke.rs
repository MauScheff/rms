use serde_json::Value;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

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

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .expect("repository root")
        .to_path_buf()
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
