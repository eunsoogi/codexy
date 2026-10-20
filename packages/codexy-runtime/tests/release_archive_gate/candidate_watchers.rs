use super::super::candidate_projection::core_assembly_fixture;
use serde_json::json;
use sha2::{Digest as _, Sha256};
use std::{fs, path::Path, process::Command};

const WATCHERS: [(&str, &str); 3] = [
    ("darwin-arm64", "bin"),
    ("linux-x86_64", "bin"),
    ("windows-x86_64", "exe"),
];

pub(super) fn materialize(
    plugin_root: &Path,
    release: &mut serde_json::Value,
    watcher_aware: bool,
) {
    // Omit the optional class when this fixture intentionally has no Watcher payloads.
    if !watcher_aware {
        release["classes"]
            .as_object_mut()
            .expect("candidate classes")
            .remove("coreWatcherMcp");
        return;
    }
    // Each core watcher entry binds its platform path, executable format, and content digest.
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
        super::make_core_candidate(&plugin, true, true);
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

#[test]
fn assembled_candidate_without_watchers_preserves_handoff_and_devtools(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = core_assembly_fixture()?;
    for (platform, extension) in WATCHERS {
        fs::remove_file(
            fixture
                .root()
                .join("staged-runtime")
                .join(format!("codexy-mcp-watcher-{platform}.{extension}")),
        )?;
    }

    let candidate = assemble_and_validate(&fixture, false)?;
    let classes = candidate["classes"].as_object().expect("candidate classes");
    assert!(classes.contains_key("devtoolsMcp"));
    assert!(classes.contains_key("coreHandoff"));
    assert!(!classes.contains_key("coreWatcherMcp"));
    Ok(())
}

#[test]
fn assembled_candidate_with_all_watchers_keeps_the_complete_class(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = core_assembly_fixture()?;
    let candidate = assemble_and_validate(&fixture, true)?;
    assert_eq!(
        candidate["classes"]["coreWatcherMcp"]["platforms"]
            .as_object()
            .expect("watcher platforms")
            .len(),
        WATCHERS.len()
    );
    Ok(())
}

#[test]
fn candidate_assembler_rejects_partial_and_nonfile_watcher_inputs(
) -> Result<(), Box<dyn std::error::Error>> {
    let partial = core_assembly_fixture()?;
    fs::remove_file(
        partial
            .root()
            .join("staged-runtime/codexy-mcp-watcher-darwin-arm64.bin"),
    )?;
    let output = partial.assemble();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("core-watcher inventory is partial"));

    let wrong_kind = core_assembly_fixture()?;
    fs::remove_file(
        wrong_kind
            .root()
            .join("staged-runtime/codexy-mcp-watcher-linux-x86_64.bin"),
    )?;
    fs::create_dir(
        wrong_kind
            .root()
            .join("staged-runtime/codexy-mcp-watcher-linux-x86_64.bin"),
    )?;
    let output = wrong_kind.assemble();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr)
        .contains("core-watcher runtime must be a regular file"));
    Ok(())
}

fn assemble_and_validate(
    fixture: &super::super::candidate_projection::CandidateAssemblyFixture,
    expect_watchers: bool,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let output = fixture.assemble();
    assert!(
        output.status.success(),
        "candidate assembly failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let root = fixture.root();
    let plugin = root.join("dist/candidate/plugins/codexy-devtools");
    let receipt_path = root.join("dist/runtime-staging-receipt.json");
    let receipt: serde_json::Value = serde_json::from_slice(&fs::read(&receipt_path)?)?;
    let candidate = receipt["candidate"].clone();
    assert_eq!(candidate["artifact"]["stagingRunId"], 1);
    assert_eq!(receipt["provenance"]["runId"], 1);
    assert_eq!(
        receipt["candidate"]["classes"]
            .get("coreWatcherMcp")
            .is_some(),
        expect_watchers
    );
    for digest in [
        receipt["artifact"]["sha256"].as_str().unwrap_or_default(),
        receipt["artifact"]["payloadManifestSha256"]
            .as_str()
            .unwrap_or_default(),
    ] {
        assert_eq!(digest.len(), 64, "staging receipt has an invalid digest");
    }

    // Verify the staged receipt binds a readable archive before its payload reaches activation.
    let archive = root.join("dist/codexy-marketplace-plugin.tar.gz");
    assert!(archive.is_file(), "assembler did not produce its archive");
    assert_eq!(
        format!("{:x}", Sha256::digest(fs::read(&archive)?)),
        receipt["artifact"]["sha256"]
    );
    let archive_read = Command::new("python3")
        .args([
            "-c",
            "import sys,tarfile; print('\\n'.join(tarfile.open(sys.argv[1]).getnames()))",
        ])
        .arg(&archive)
        .output()?;
    assert!(
        archive_read.status.success(),
        "archive is not readable as tar.gz"
    );
    assert!(String::from_utf8_lossy(&archive_read.stdout)
        .contains("plugins/codexy-devtools/runtime-candidate.json"));

    // The private activation path must preserve candidate digests and the optional Watcher class.
    let selected = root.join("selected-candidate.json");
    let protected = root.join("protected.json");
    let materialized = Command::new("python3")
        .arg(codexy_runtime::paths::repository_root().join("scripts/materialize_runtime_source.py"))
        .current_dir(root)
        .env("SOURCE_PLUGIN", &plugin)
        .env("STAGED_PLUGIN", &plugin)
        .env("PUBLIC_RELEASE", "0")
        .env("ACTIVATION_RECORD", &receipt_path)
        .env(
            "STAGING_SOURCE_COMMIT",
            candidate["source"]["commit"]
                .as_str()
                .expect("source commit"),
        )
        .env("STAGING_RUN_ID", "1")
        .env(
            "STAGED_SHA",
            receipt["artifact"]["sha256"]
                .as_str()
                .expect("archive digest"),
        )
        .env("RELEASE_TAG", "v1.6.0")
        .env("SELECTED_CANDIDATE", &selected)
        .env("LEGACY_DISPATCHER_FREE", "0")
        .env("PROTECTED", &protected)
        .output()?;
    assert!(
        materialized.status.success(),
        "generated candidate failed source materialization: {}",
        String::from_utf8_lossy(&materialized.stderr)
    );
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&fs::read(selected)?)?,
        candidate
    );
    let protected: serde_json::Value = serde_json::from_slice(&fs::read(protected)?)?;
    assert!(protected.get("handoff-runtime.json").is_some());
    assert_eq!(
        protected
            .as_object()
            .expect("protected payload hashes")
            .keys()
            .any(|path| path.contains("codexy-mcp-watcher-")),
        expect_watchers
    );
    Ok(candidate)
}
