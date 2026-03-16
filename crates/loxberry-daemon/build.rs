use std::env;
use std::process::Command;

fn main() {
    // Always re-run if git changes (only when .git is available)
    println!("cargo:rerun-if-changed=../../.git/HEAD");
    println!("cargo:rerun-if-changed=../../.git/refs");
    println!("cargo:rerun-if-env-changed=GIT_HASH");
    println!("cargo:rerun-if-env-changed=GIT_TAG");
    println!("cargo:rerun-if-env-changed=GIT_DIRTY");

    // Try to get git info from environment (set by Docker build) or from git commands
    let git_hash = env::var("GIT_HASH")
        .ok()
        .or_else(|| {
            Command::new("git")
                .args(["rev-parse", "--short=8", "HEAD"])
                .output()
                .ok()
                .and_then(|output| {
                    if output.status.success() {
                        String::from_utf8(output.stdout).ok()
                    } else {
                        None
                    }
                })
        })
        .unwrap_or_else(|| "unknown".to_string())
        .trim()
        .to_string();

    // Get git tag (if on a tag)
    let git_tag = env::var("GIT_TAG")
        .ok()
        .filter(|s| !s.is_empty())
        .or_else(|| {
            Command::new("git")
                .args(["describe", "--exact-match", "--tags", "HEAD"])
                .output()
                .ok()
                .and_then(|output| {
                    if output.status.success() {
                        String::from_utf8(output.stdout).ok()
                    } else {
                        None
                    }
                })
        })
        .map(|s| s.trim().to_string());

    // Check if working directory is dirty
    let git_dirty = env::var("GIT_DIRTY")
        .ok()
        .map(|s| s == "true")
        .unwrap_or_else(|| {
            Command::new("git")
                .args(["diff", "--quiet"])
                .status()
                .map(|status| !status.success())
                .unwrap_or(false)
        });

    // Set environment variables for compile-time
    println!("cargo:rustc-env=GIT_HASH={}", git_hash);

    if let Some(ref tag) = git_tag {
        println!("cargo:rustc-env=GIT_TAG={}", tag);
    }

    if git_dirty {
        println!("cargo:rustc-env=GIT_DIRTY=true");
    }

    // Build full version string
    let pkg_version = env!("CARGO_PKG_VERSION");
    let version_string = if let Some(ref tag) = git_tag {
        // On a tag: just show the tag (should match pkg_version)
        tag.clone()
    } else {
        // Not on a tag: show version + commit
        let dirty_suffix = if git_dirty { "-dirty" } else { "" };
        format!("{}+{}{}", pkg_version, git_hash, dirty_suffix)
    };

    println!("cargo:rustc-env=BUILD_VERSION={}", version_string);
}
