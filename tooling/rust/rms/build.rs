use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=../../../.git/HEAD");
    println!("cargo:rerun-if-changed=../../../.git/index");
    let revision = Command::new("git")
        .args(["rev-parse", "--short=12", "HEAD"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .filter(|revision| !revision.is_empty())
        .unwrap_or_else(|| "source-package".to_string());
    println!("cargo:rustc-env=RMS_BUILD_REVISION={revision}");
}
