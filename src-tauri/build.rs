use std::process::Command;

fn main() {
    tauri_build::build();

    // Embed git hash (short, 8 chars) + ISO date for the version display.
    // Fall back gracefully when git isn't available (e.g. zip download).
    let hash = Command::new("git")
        .args(["rev-parse", "--short=8", "HEAD"])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                String::from_utf8(o.stdout).ok()
            } else {
                None
            }
        })
        .unwrap_or_else(|| "unknown".to_string())
        .trim()
        .to_string();

    let date = Command::new("git")
        .args(["log", "-1", "--format=%cs"]) // YYYY-MM-DD of last commit
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                String::from_utf8(o.stdout).ok()
            } else {
                None
            }
        })
        .unwrap_or_else(|| "unknown".to_string())
        .trim()
        .to_string();

    println!("cargo:rustc-env=OPENSPLIT_GIT_HASH={hash}");
    println!("cargo:rustc-env=OPENSPLIT_BUILD_DATE={date}");

    // Re-run this script if HEAD changes.
    println!("cargo:rerun-if-changed=../.git/HEAD");
    println!("cargo:rerun-if-changed=../.git/refs/heads");
}
