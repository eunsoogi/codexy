use super::super::candidate_projection::core_assembly_fixture;
use serde_json::json;
use sha2::{Digest as _, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

const WATCHERS: [(&str, &str, &str); 3] = [
    ("darwin-arm64", "bin", "mach-o"),
    ("linux-x86_64", "bin", "elf"),
    ("windows-x86_64", "exe", "pe"),
];

fn watcher_path(root: &Path, platform: &str) -> PathBuf {
    let extension = WATCHERS
        .iter()
        .find(|(known, _, _)| *known == platform)
        .expect("known watcher platform")
        .1;
    root.join("staged-runtime")
        .join(format!("codexy-mcp-watcher-{platform}.{extension}"))
}

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
    for (platform, extension, kind) in WATCHERS {
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
fn assembled_candidate_without_watchers_preserves_handoff_and_devtools(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = core_assembly_fixture()?;
    for (platform, _, _) in WATCHERS {
        fs::remove_file(watcher_path(fixture.root(), platform))?;
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
    fs::remove_file(watcher_path(partial.root(), "darwin-arm64"))?;
    let output = partial.assemble();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("core-watcher inventory is partial"));

    let wrong_kind = core_assembly_fixture()?;
    let path = watcher_path(wrong_kind.root(), "linux-x86_64");
    fs::remove_file(&path)?;
    fs::create_dir(path)?;
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
    let extracted = root.join("extracted");
    let extracted_plugin = extracted.join("plugins/codexy-devtools");
    // Exercise both archive readers on the assembler's bytes without launching fixture MCP runtimes.
    let archive_read = Command::new("sh")
        .arg("-c")
        .arg(
            "mkdir -p \"$2\" && tar -xzf \"$1\" -C \"$2\" && python3 \"$3\" staged \"$4\" && \"$5\" --plugin-root \"$4\" --check-runtime-artifacts",
        )
        .arg("--")
        .arg(&archive)
        .arg(&extracted)
        .arg(codexy_runtime::paths::repository_root().join("scripts/inspect-release-archive-contract.py"))
        .arg(&extracted_plugin)
        .arg(env!("CARGO_BIN_EXE_codexy-validate"))
        .output()?;
    assert!(
        archive_read.status.success(),
        "generated archive failed Python/Rust readers: {}",
        String::from_utf8_lossy(&archive_read.stderr)
    );

    // The private activation path must preserve candidate digests and the optional Watcher class.
    let selected = root.join("selected-candidate.json");
    let protected = root.join("protected.json");
    let mut materializer = Command::new("python3");
    materializer
        .arg(codexy_runtime::paths::repository_root().join("scripts/materialize_runtime_source.py"))
        .current_dir(root)
        .env("SOURCE_PLUGIN", &plugin)
        .env("STAGED_PLUGIN", &extracted_plugin)
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
        .env("PROTECTED", &protected);
    let materialized = materializer.output()?;
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
    if !expect_watchers {
        // A copied generated candidate with a corrupt schema must fail the activation identity check.
        let candidate_path = extracted_plugin.join("runtime-candidate.json");
        let mut malformed: serde_json::Value = serde_json::from_slice(&fs::read(&candidate_path)?)?;
        malformed["schema"] = json!("codexy-runtime-candidate/invalid");
        fs::write(&candidate_path, serde_json::to_vec(&malformed)?)?;
        let rejected = materializer.output()?;
        assert!(
            !rejected.status.success(),
            "accepted malformed generated candidate"
        );
    }
    Ok(candidate)
}

pub(crate) fn seed_reader_runtime_inputs(root: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let binaries = super::complete_plugin_fixture(&root.join("reader-fixtures"))?;
    super::make_core_candidate(&binaries, true, false);
    let staged = root.join("staged-runtime");
    // Reuse valid runtime signatures so readers validate the archive without executing these fixtures.
    super::super::candidate_projection::copy_fixture_tree(&binaries.join("runtime"), &staged)?;
    fs::copy(
        binaries.join("mcp/codexy-mcp-devtools.exe"),
        staged.join("codexy-mcp-devtools-windows-x86_64.exe"),
    )?;
    Ok(())
}
