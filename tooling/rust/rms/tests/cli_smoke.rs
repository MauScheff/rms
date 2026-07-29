use std::process::Command;

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
