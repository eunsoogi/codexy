use std::fs;

use serde_json::Value;
use sha2::{Digest as _, Sha256};

use crate::support::FixtureCommand as Command;

use super::final_archive_fixture::FinalArchiveFixture;

const STAGING_COMMIT: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const ACTIVATION_COMMIT: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

#[test]
fn materializer_projects_an_immutable_legacy_core_candidate_into_devtools()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = FinalArchiveFixture::new()?;
    let legacy = fixture.root.join("legacy input");
    fs::create_dir(&legacy)?;
    assert!(Command::new("tar")
        .args(["-xzf"])
        .arg(&fixture.staged_archive)
        .arg("-C")
        .arg(&legacy)
        .status()?
        .success());
    let legacy_plugin = legacy.join("plugins/codexy-devtools");
    fs::remove_file(legacy_plugin.join("mcp/codexy-mcp-devtools.exe"))?;
    for server in ["lsp", "codegraph"] {
        fs::write(
            legacy_plugin.join(format!("mcp/codexy-mcp-{server}.exe")),
            fs::read(legacy_plugin.join(format!(
                "runtime/codexy-mcp-{server}-windows-x86_64.exe"
            )))?,
        )?;
    }
    fs::rename(
        legacy_plugin,
        legacy.join("plugins/codexy"),
    )?;
    let archive = fixture.root.join("legacy-core.tar.gz");
    assert!(Command::new("tar")
        .env("COPYFILE_DISABLE", "1")
        .args(["-C"])
        .arg(&legacy)
        .args(["-czf"])
        .arg(&archive)
        .arg("plugins/codexy")
        .status()?
        .success());
    let input_entries = Command::new("tar")
        .args(["-tzf"])
        .arg(&archive)
        .output()?;
    assert!(input_entries.status.success());
    let input_entries = String::from_utf8(input_entries.stdout)?;
    for server in ["lsp", "codegraph"] {
        assert!(input_entries.contains(&format!("mcp/codexy-mcp-{server}.exe")));
    }
    assert!(!input_entries.contains("mcp/codexy-mcp-devtools.exe"));
    let activation = fixture.root.join(".agents/plugins/runtime-activation.json");
    let mut record: Value = serde_json::from_slice(&fs::read(&activation)?)?;
    record["artifact"]["sha256"] = Value::String(format!("{:x}", Sha256::digest(fs::read(&archive)?)));
    fs::write(&activation, serde_json::to_vec(&record)?)?;

    let output = Command::new(
        codexy_runtime::paths::repository_root()
            .join("scripts/materialize-runtime-release-archive"),
    )
    .arg_path(&archive)
    .arg_path(&fixture.final_archive)
    .current_dir(&fixture.root)
    .env("RELEASE_TAG", "v1.3.0")
    .env("STAGING_SOURCE_COMMIT", STAGING_COMMIT)
    .env("ACTIVATION_COMMIT", ACTIVATION_COMMIT)
    .env("STAGING_RUN_ID", "42")
    .output()?;
    assert!(
        output.status.success(),
        "legacy core projection failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let entries = Command::new("tar")
        .args(["-tzf"])
        .arg(&fixture.final_archive)
        .output()?;
    assert!(entries.status.success());
    let entries = String::from_utf8(entries.stdout)?;
    assert!(entries.lines().all(|path| !path.starts_with("plugins/codexy/")));
    assert!(entries.lines().any(|path| path == "plugins/codexy-devtools/runtime/"));
    assert!(entries.lines().all(|path| !path.ends_with("windows-x86_64.exe")));
    assert!(entries.lines().all(|path| !path.ends_with("mcp/codexy-mcp-lsp.exe")));
    assert!(entries.lines().all(|path| !path.ends_with("mcp/codexy-mcp-codegraph.exe")));
    assert!(entries.lines().all(|path| !path.ends_with("mcp/codexy-mcp-devtools.exe")));
    let extracted = fixture.root.join("legacy output");
    fs::create_dir(&extracted)?;
    assert!(Command::new("tar")
        .args(["-xzf"])
        .arg(&fixture.final_archive)
        .arg("-C")
        .arg(&extracted)
        .status()?
        .success());
    let projected = extracted.join("plugins/codexy-devtools");
    assert!(projected.join("mcp/codexy-mcp-devtools").is_file());
    let manifest: Value = serde_json::from_slice(&fs::read(projected.join(".codex-plugin/plugin.json"))?)?;
    assert_eq!(manifest["supportedPlatforms"], serde_json::json!(["darwin-arm64", "linux-x86_64"]));
    let contract = Command::new("python3")
        .arg(
            codexy_runtime::paths::repository_root()
                .join("scripts/inspect-release-archive-contract.py"),
        )
        .args(["public-release"])
        .arg_path(&projected)
        .output()?;
    assert!(
        contract.status.success(),
        "legacy projection violated the public archive contract: {}",
        String::from_utf8_lossy(&contract.stderr)
    );
    Ok(())
}

#[test]
fn materializer_rejects_a_dispatcher_free_devtools_archive()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = FinalArchiveFixture::new()?;
    let input = fixture.root.join("dispatcher-free devtools input");
    fs::create_dir(&input)?;
    assert!(Command::new("tar")
        .args(["-xzf"])
        .arg(&fixture.staged_archive)
        .arg("-C")
        .arg(&input)
        .status()?
        .success());
    fs::remove_file(input.join("plugins/codexy-devtools/mcp/codexy-mcp-devtools.exe"))?;
    let archive = fixture.root.join("dispatcher-free-devtools.tar.gz");
    assert!(Command::new("tar")
        .env("COPYFILE_DISABLE", "1")
        .args(["-C"])
        .arg(&input)
        .args(["-czf"])
        .arg(&archive)
        .arg("plugins/codexy-devtools")
        .status()?
        .success());
    let activation = fixture.root.join(".agents/plugins/runtime-activation.json");
    let mut record: Value = serde_json::from_slice(&fs::read(&activation)?)?;
    record["artifact"]["sha256"] = Value::String(format!("{:x}", Sha256::digest(fs::read(&archive)?)));
    fs::write(&activation, serde_json::to_vec(&record)?)?;
    let output = Command::new(
        codexy_runtime::paths::repository_root()
            .join("scripts/materialize-runtime-release-archive"),
    )
    .arg_path(&archive)
    .arg_path(&fixture.final_archive)
    .current_dir(&fixture.root)
    .env("RELEASE_TAG", "v1.3.0")
    .env("STAGING_SOURCE_COMMIT", STAGING_COMMIT)
    .env("ACTIVATION_COMMIT", ACTIVATION_COMMIT)
    .env("STAGING_RUN_ID", "42")
    .output()?;
    assert!(!output.status.success(), "dispatcher-free devtools archive was accepted");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("selected Windows dispatcher missing"),
        "unexpected materializer stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn legacy_input_detection_rejects_a_mixed_plugin_archive()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = FinalArchiveFixture::new()?;
    let root = fixture.root.join("mixed input");
    fs::create_dir(&root)?;
    assert!(Command::new("tar")
        .args(["-xzf"])
        .arg(&fixture.staged_archive)
        .arg("-C")
        .arg(&root)
        .status()?
        .success());
    let core = root.join("plugins/codexy");
    fs::create_dir_all(&core)?;
    fs::write(core.join("unexpected"), b"mixed root\n")?;
    let archive = fixture.root.join("mixed.tar.gz");
    assert!(Command::new("tar")
        .env("COPYFILE_DISABLE", "1")
        .args(["-C"])
        .arg(&root)
        .args(["-czf"])
        .arg(&archive)
        .args(["plugins/codexy-devtools", "plugins/codexy"])
        .status()?
        .success());

    let output = Command::new(
        codexy_runtime::paths::repository_root().join("scripts/check-release-archive-entries"),
    )
    .arg_path(&archive)
    .args(["52428800", "10000", "268435456", "--input-plugin-root", "auto"])
    .output()?;
    assert!(!output.status.success(), "mixed plugin archive was accepted");
    assert!(String::from_utf8_lossy(&output.stderr).contains("unsafe archive path"));
    Ok(())
}
