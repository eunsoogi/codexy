use serde_json::json;
use sha2::{Digest as _, Sha256};
use std::path::Path;

pub(super) fn materialize(plugin_root: &Path, release: &mut serde_json::Value) {
    let host = crate::support::release_archive::fixture_host_platform(
        std::env::consts::OS,
        std::env::consts::ARCH,
    )
    .expect("host platform");
    for (platform, extension, kind) in [
        ("darwin-arm64", "bin", "mach-o"),
        ("linux-x86_64", "bin", "elf"),
        ("windows-x86_64", "exe", "pe"),
    ] {
        let relative = format!("runtime/codexy-mcp-watcher-{platform}.{extension}");
        let path = plugin_root.join(&relative);
        if platform == host {
            std::fs::copy(env!("CARGO_BIN_EXE_codexy-mcp-watcher"), &path)
                .expect("native watcher runtime");
        } else {
            std::fs::write(&path, format!("fixture core watcher {platform}\n"))
                .expect("watcher runtime");
        }
        if extension == "bin" || platform == host {
            super::make_executable(&path).expect("executable watcher");
        }
        release["classes"]["coreWatcherMcp"]["platforms"][platform] = json!({
            "path": relative, "kind": kind,
            "sha256": format!("{:x}", Sha256::digest(std::fs::read(&path).expect("watcher bytes"))),
        });
    }
}

#[test]
fn candidate_archive_rejects_each_missing_watcher_platform() {
    for platform in ["darwin-arm64", "linux-x86_64", "windows-x86_64"] {
        let root = super::tempdir().expect("candidate root");
        let plugin = super::complete_plugin_fixture(root.path()).expect("plugin");
        super::make_candidate_proven_windows_package_with_core(&plugin, true);
        let extension = if platform == "windows-x86_64" {
            "exe"
        } else {
            "bin"
        };
        let name = format!("codexy-mcp-watcher-{platform}.{extension}");
        std::fs::remove_file(plugin.join("runtime").join(&name)).expect("remove watcher");
        let archive = root.path().join("missing-watcher.tar.gz");
        super::create_archive(root.path(), &archive).expect("archive");
        let output = super::run_candidate_gate(root.path(), &archive, &plugin);
        assert!(!output.status.success(), "accepted missing {platform}");
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .contains(&format!("missing core watcher runtime: {name}")),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
