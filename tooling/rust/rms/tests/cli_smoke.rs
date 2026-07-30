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
    assert_eq!(
        String::from_utf8(output.stdout).expect("UTF-8 version output"),
        format!("rms {}\n", env!("CARGO_PKG_VERSION"))
    );
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
        "examples/probes/series-faults.yaml",
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
