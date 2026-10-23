use super::super::candidate_projection::{
    CandidateAssemblyFixture, CandidateTestResult, core_assembly_fixture,
};
use serde_json::json;
use sha2::{Digest as _, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

pub(crate) const WATCHERS: [(&str, &str, &str); 3] = [
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
fn assembled_candidates_support_complete_or_absent_watchers() -> CandidateTestResult {
    for expect_watchers in [false, true] {
        let fixture = core_assembly_fixture()?;
        if !expect_watchers {
            for (platform, _, _) in WATCHERS {
                fs::remove_file(watcher_path(fixture.root(), platform))?;
            }
        }
        let candidate = assemble_and_validate(&fixture, expect_watchers)?;
        let classes = candidate["classes"].as_object().expect("candidate classes");
        assert!(classes.contains_key("devtoolsMcp"));
        assert!(classes.contains_key("coreHandoff"));
        assert_eq!(classes.contains_key("coreWatcherMcp"), expect_watchers);
        if expect_watchers {
            assert_eq!(
                classes["coreWatcherMcp"]["platforms"]
                    .as_object()
                    .unwrap()
                    .len(),
                WATCHERS.len()
            );
        }
    }
    Ok(())
}

#[test]
fn candidate_assembler_rejects_invalid_watcher_inputs() -> CandidateTestResult {
    let partial = core_assembly_fixture()?;
    fs::remove_file(watcher_path(partial.root(), "darwin-arm64"))?;
    assert_rejected(&partial, "core-watcher inventory is partial");

    let wrong_kind = core_assembly_fixture()?;
    let path = watcher_path(wrong_kind.root(), "linux-x86_64");
    fs::remove_file(&path)?;
    fs::create_dir(path)?;
    assert_rejected(&wrong_kind, "core-watcher runtime must be a regular file");
    #[cfg(unix)]
    {
        let symlink = core_assembly_fixture()?;
        let path = watcher_path(symlink.root(), "darwin-arm64");
        fs::remove_file(&path)?;
        // Reject links before hashing so external or missing targets cannot enter the candidate.
        std::os::unix::fs::symlink("missing-watcher", &path)?;
        assert_rejected(&symlink, "core-watcher runtime must be a regular file");
    }
    Ok(())
}

fn assert_rejected(fixture: &CandidateAssemblyFixture, reason: &str) {
    let output = fixture.assemble();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains(reason));
}

fn assemble_and_validate(
    fixture: &super::super::candidate_projection::CandidateAssemblyFixture,
    expect_watchers: bool,
) -> CandidateTestResult<serde_json::Value> {
    let output = fixture.assemble();
    assert!(
        output.status.success(),
        "candidate assembly failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let root = fixture.root();
    let target_version =
        super::super::candidate_projection::candidate_admission::candidate_version(root)?;
    let receipt_path = root.join("dist/runtime-staging-receipt.json");
    let receipt: serde_json::Value = serde_json::from_slice(&fs::read(&receipt_path)?)?;
    let candidate = receipt["candidate"].clone();
    // Verify the staged receipt binds a readable archive before its payload reaches activation.
    let archive = root.join("dist/codexy-marketplace-plugin.tar.gz");
    assert_eq!(
        format!("{:x}", Sha256::digest(fs::read(&archive)?)),
        receipt["artifact"]["sha256"]
    );
    let extracted = root.join("extracted");
    let extracted_plugin = extracted.join("plugins/codexy-devtools");
    fs::create_dir_all(&extracted)?;
    let extract = Command::new("tar")
        .args(["-xzf"])
        .arg(&archive)
        .arg("-C")
        .arg(&extracted)
        .status()?;
    assert!(extract.success(), "could not extract generated candidate");
    let candidate_bytes = fs::read(extracted_plugin.join("runtime-candidate.json"))?;
    assert_eq!(
        format!("{:x}", Sha256::digest(&candidate_bytes)),
        receipt["artifact"]["payloadManifestSha256"]
    );
    let mut reader_receipt = receipt.clone();
    reader_receipt["candidate"] = serde_json::from_slice(&candidate_bytes)?;
    assert_eq!(reader_receipt["candidate"], candidate);
    let reader_receipt_path = root.join("dist/archive-backed-receipt.json");
    fs::write(&reader_receipt_path, serde_json::to_vec(&reader_receipt)?)?;

    if !expect_watchers {
        // Refresh the payload digest so this counterexample reaches Rust schema validation.
        let mut malformed = reader_receipt.clone();
        malformed["candidate"]["schema"] = json!("codexy-runtime-candidate/invalid");
        let malformed_bytes = serde_json::to_vec(&malformed["candidate"])?;
        malformed["artifact"]["payloadManifestSha256"] =
            json!(format!("{:x}", Sha256::digest(malformed_bytes)));
        let malformed_path = root.join("dist/malformed-candidate-receipt.json");
        fs::write(&malformed_path, serde_json::to_vec(&malformed)?)?;
        let rejected = codexy_runtime::version::activation::activate(
            fixture.root(),
            &target_version,
            &malformed_path,
        )
        .expect_err("accepted malformed candidate schema");
        assert!(
            rejected.to_string().contains("candidate schema"),
            "{rejected}"
        );
    }
    // The Rust activation API validates archive-backed staging receipts and optional class sets.
    codexy_runtime::version::activation::activate(
        fixture.root(),
        &target_version,
        &reader_receipt_path,
    )?;

    // The private activation path must preserve candidate digests and the optional Watcher class.
    let selected = root.join("selected-candidate.json");
    let protected = root.join("protected.json");
    let mut materializer = Command::new("python3");
    materializer
        .arg(codexy_runtime::paths::repository_root().join("scripts/materialize_runtime_source.py"))
        .current_dir(root)
        // Source metadata stays selected while the staged candidate advances.
        .env("SOURCE_PLUGIN", root.join("plugins/codexy-devtools"))
        .env("STAGED_PLUGIN", &extracted_plugin)
        .env("PUBLIC_RELEASE", "0")
        .env("ACTIVATION_RECORD", &reader_receipt_path)
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
        .env("RELEASE_TAG", format!("v{}", env!("CARGO_PKG_VERSION")))
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
    Ok(candidate)
}
