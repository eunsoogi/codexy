use std::fs;

use serde_json::{Value, json};

use crate::support::{self, FixtureCommand as Command};

use super::{final_archive_fixture::FinalArchiveFixture, workflow};

#[test]
fn final_publisher_materializes_and_exercises_the_public_archive()
-> Result<(), Box<dyn std::error::Error>> {
    let publisher = workflow("publish-version-release.yml")?;
    let run = publisher.1;
    let inputs = publisher.2["on"]["workflow_dispatch"]["inputs"]
        .as_mapping()
        .ok_or("final publisher dispatch inputs")?;
    for input in ["staging_source_commit", "activation_commit", "staging_run_id"] {
        assert!(inputs.contains_key(input), "final publisher lacks {input}");
    }
    support::assert_structured_literals(
        &run,
        "final publisher lineage and archive contract",
        &[
            "STAGING_SOURCE_COMMIT",
            "ACTIVATION_COMMIT",
            "git fetch --no-tags origin +refs/heads/main:refs/remotes/origin/main",
            "test \"$ACTIVATION_COMMIT\" = \"$(git rev-parse origin/main)\"",
            "scripts/materialize-runtime-release-archive",
            "codexy-runtime-package.tar.gz",
            "runtime-release-receipt.json",
            "scripts/inspect-release-archive public.tar.gz public-inspect/plugins/codexy",
            "gh attestation verify public-runtime.tar.gz",
            "gh release view v1.3.0",
            "gh release upload v1.3.0",
            "--draft",
            "gh release edit v1.3.0 --draft=false",
            "gh release download v1.3.0",
            "release asset differs from verified bytes",
            "--plugin-root \"$PWD/plugins/codexy\"",
        ],
    );
    support::assert_structured_absent_literals(
        &run,
        "immutable release asset reconciliation",
        &["--clobber"],
    );
    Ok(())
}

#[test]
fn materializer_preserves_staged_runtime_with_space_safe_paths_without_rsync()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = FinalArchiveFixture::new()?;
    let missing_rsync = fixture.root.join("missing rsync command");
    fs::create_dir(&missing_rsync)?;
    let rsync = missing_rsync.join("rsync");
    fs::write(&rsync, "#!/bin/sh\nexit 127\n")?;
    support::make_executable(&rsync)?;
    let output = fixture.materialize(Some(missing_rsync))?;
    assert!(
        output.status.success(),
        "materializer must preserve the archive without rsync: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        fixture.root.file_name().and_then(|path| path.to_str()),
        Some("final archive fixture with spaces")
    );
    let extraction = tempfile::tempdir()?;
    let extracted = extraction.path().join("extracted");
    fs::create_dir(&extracted)?;
    assert!(
        Command::new("tar")
            .args(["-xzf"])
            .arg(&fixture.final_archive)
            .arg("-C")
            .arg(&extracted)
            .status()?
            .success()
    );
    let plugin = extracted.join("plugins/codexy");
    let manifest: Value =
        serde_json::from_slice(&fs::read(plugin.join(".codex-plugin/plugin.json"))?)?;
    assert_eq!(manifest["version"], "1.3.0");
    assert_eq!(
        manifest["supportedPlatforms"],
        json!(["darwin-arm64", "linux-x86_64", "windows-x86_64"])
    );
    assert!(!plugin.join("runtime-release.json").exists());
    assert!(!plugin.join("runtime-candidate.json").exists());
    for server in ["lsp", "codegraph"] {
        let wrapper = fs::read_to_string(plugin.join(format!("mcp/codexy-mcp-{server}")))?;
        support::assert_structured_literals(
            &wrapper,
            &format!("final archive {server} wrapper pin"),
            &["getcodexy==1.3.0"],
        );
        support::assert_structured_absent_literals(
            &wrapper,
            &format!("final archive {server} wrapper must not retain prior pin"),
            &["getcodexy==1.2.2"],
        );
    }
    let runtime = plugin.join("runtime/codexy-mcp-lsp-darwin-arm64.bin");
    assert_eq!(fs::read(&runtime)?, fixture.runtime);
    let smoke = Command::new(runtime).arg("--help").output()?;
    assert!(smoke.status.success());
    assert_eq!(smoke.stdout, b"final archive runtime\n");
    for server in ["lsp", "codegraph"] {
        let runtime = plugin.join(format!("runtime/codexy-mcp-{server}-windows-x86_64.exe"));
        assert_eq!(fs::read(plugin.join(format!("mcp/codexy-mcp-{server}.exe")))?, fs::read(runtime)?);
    }
    Ok(())
}
