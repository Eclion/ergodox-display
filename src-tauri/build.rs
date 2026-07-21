use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn main() {
    // Embed build metadata for the settings window's info line.
    let sha = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .filter(|out| out.status.success())
        .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_string())
        .unwrap_or_else(|| "unknown".into());
    println!("cargo:rustc-env=BUILD_GIT_SHA={sha}");

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    println!("cargo:rustc-env=BUILD_TIMESTAMP={timestamp}");
    println!("cargo:rerun-if-changed=../.git/HEAD");

    tauri_build::build()
}
