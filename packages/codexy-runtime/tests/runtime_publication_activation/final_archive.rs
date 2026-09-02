use std::fs;

use serde_json::{Value, json};

use crate::support::{self, FixtureCommand as Command};

use super::{final_archive_fixture::{FinalArchiveFixture, RUNTIME}, workflow};

#[test]
fn final_publisher_materializes_and_exercises_the_public_archive()
-> Result<(), Box<dyn std::error::Error>> {
    let publisher = workflow("publish-version-release.yml")?;
    let run = format!(
        "{}\n{}\n{}",
        publisher.1,
        fs::read_to_string(codexy_runtime::paths::repository_root().join("scripts/publish-verified-release"))?,
        fs::read_to_string(codexy_runtime::paths::repository_root().join("scripts/finalize-verified-release"))?,
    );
    support::assert_structured_literals(
        &run,
        "final publisher lineage and archive contract",
        &[
            "STAGING_SOURCE_COMMIT",
            "ACTIVATION_COMMIT",
            "git fetch --no-tags origin +refs/heads/main:refs/remotes/origin/main",
            "test \"$ACTIVATION_COMMIT\" = \"$(git rev-parse origin/main)\"",
            "scripts/materialize-runtime-release-archive",
            "scripts/assemble-release-train-archive.sh",
            "codexy-marketplace-bundle.tar.gz",
            "cp staging/codexy-marketplace-plugin.tar.gz dist/codexy-runtime-package.tar.gz",
            "runtime-release-receipt.json",
            "scripts/inspect-release-archive public.tar.gz public-inspect/plugins/codexy-devtools",
            "scripts/verify-release-attestation-set",
            "gh release view \"$RELEASE_TAG\"",
            "-F draft=true",
            "RELEASE_ID", "gh api --method POST", "uploadUrl: .upload_url", "release_upload_url", "\"$upload_url?name=$asset\"",
            "releases/assets/$asset_id", "gh api --method PATCH",
            "release asset differs from verified bytes",
            "--plugin-root \"$PWD/plugins/codexy-devtools\"",
            "jq -er .version\n          )\" = \"$TARGET_VERSION\"",
        ],
    );
    support::assert_structured_absent_literals(
        &run,
        "immutable release asset reconciliation",
        &["--clobber", "cp dist/codexy-marketplace-plugin.tar.gz dist/codexy-runtime-package.tar.gz", "gh release upload \"$RELEASE_TAG\"", "gh release download \"$RELEASE_TAG\"", "releases/tags/$RELEASE_TAG"],
    );
    let marker = |needle: &str| run.find(needle).ok_or("publisher ordering");
    let staged_identity = marker("tar -xOzf staging/codexy-marketplace-plugin.tar.gz")?;
    let runtime_copy = marker("cp staging/codexy-marketplace-plugin.tar.gz")?;
    let public_materialization = marker("scripts/materialize-runtime-release-archive")?;
    assert!(
        staged_identity < runtime_copy && runtime_copy < public_materialization,
        "staged identity must be checked before the byte-preserving copy and public materialization"
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
    let plugin = extracted.join("plugins/codexy-devtools");
    let manifest: Value =
        serde_json::from_slice(&fs::read(plugin.join(".codex-plugin/plugin.json"))?)?;
    assert_eq!(manifest["version"], "1.3.0");
    assert_eq!(
        manifest["supportedPlatforms"],
        json!(["darwin-arm64", "linux-x86_64", "windows-x86_64"])
    );
    assert!(!plugin.join("runtime-release.json").exists());
    assert!(!plugin.join("runtime-candidate.json").exists());
    let wrapper = fs::read_to_string(plugin.join("mcp/codexy-mcp-devtools"))?;
    support::assert_structured_literals(&wrapper, "final archive shared wrapper pin", &["getcodexy==1.3.0"]);
    support::assert_structured_absent_literals(&wrapper, "final archive shared wrapper must not retain prior pin", &["getcodexy==1.2.2"]);
    let runtime = plugin.join("runtime/codexy-mcp-lsp-darwin-arm64.bin");
    assert_eq!(fs::read(&runtime)?, RUNTIME);
    let smoke = Command::new(runtime).arg("--help").output()?;
    assert!(smoke.status.success());
    assert_eq!(smoke.stdout, b"final archive runtime\n");
    assert!(plugin.join("mcp/codexy-mcp-devtools.exe").is_file());
    assert!(!plugin.join("mcp/codexy-mcp-lsp.exe").exists());
    assert!(!plugin.join("mcp/codexy-mcp-codegraph.exe").exists());
    for server in ["lsp", "codegraph"] {
        assert_eq!(
            fs::read(plugin.join(format!("mcp/codexy-mcp-{server}.cmd")))?,
            format!(
                "@echo off\n\"%~dp0codexy-mcp-devtools.exe\" {server} %*\nexit /b %ERRORLEVEL%\n"
            )
            .as_bytes(),
            "public archive must retain the non-native {server} Windows delegate"
        );
    }
    Ok(())
}

#[test]
fn materializer_projects_current_source_onto_an_immutable_public_runtime()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = FinalArchiveFixture::new()?;
    fs::write(
        fixture.root.join(".agents/plugins/runtime-activation.json"),
        b"stale activation metadata must not be read for public assembly\n",
    )?;
    let input_tree = fixture.input_tree()?;
    let output = fixture.materialize_public()?;
    assert!(
        output.status.success(),
        "public projection failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let extraction = tempfile::tempdir()?;
    assert!(
        Command::new("tar")
            .args(["-xzf"])
            .arg(&fixture.final_archive)
            .arg("-C")
            .arg(extraction.path())
            .status()?
            .success()
    );
    let plugin = extraction.path().join("plugins/codexy-devtools");
    let public_extraction = tempfile::tempdir()?;
    assert!(
        Command::new("tar")
            .args(["-xzf"])
            .arg(&fixture.public_archive)
            .arg("-C")
            .arg(public_extraction.path())
            .status()?
            .success()
    );
    let public_plugin = public_extraction.path().join("plugins/codexy-devtools");
    for platform in ["darwin-arm64", "linux-x86_64", "windows-x86_64"] {
        let extension = if platform == "windows-x86_64" { "exe" } else { "bin" };
        for server in ["lsp", "codegraph"] {
            let runtime = format!("runtime/codexy-mcp-{server}-{platform}.{extension}");
            assert_eq!(
                fs::read(plugin.join(&runtime))?,
                fs::read(public_plugin.join(runtime))?,
                "public projection must preserve the immutable {platform} {server} runtime"
            );
        }
    }
    assert!(plugin.join("mcp/codexy-mcp-devtools.exe").is_file());
    assert!(!plugin.join("mcp/codexy-mcp-lsp.exe").exists());
    assert!(!plugin.join("mcp/codexy-mcp-codegraph.exe").exists());
    for server in ["lsp", "codegraph"] {
        assert!(plugin.join(format!("mcp/codexy-mcp-{server}.cmd")).is_file());
    }
    assert_eq!(
        fs::read(plugin.join("hooks/current-policy.txt"))?,
        b"current policy\n",
        "public projection must replace stale non-runtime source"
    );
    let inventory = extraction.path().join("mcp-entrypoints");
    fs::write(
        &inventory,
        "mcp/codexy-mcp-devtools\n",
    )?;
    let inspection = Command::new(
        codexy_runtime::paths::repository_root()
            .join("scripts/check-release-archive-entries"),
    )
    .arg_path(&fixture.final_archive)
    .args(["52428800", "10000", "268435456"])
    .arg_path(&inventory)
    .output()?;
    assert!(
        inspection.status.success(),
        "public projection must preserve executable MCP wrapper archive modes: {}",
        String::from_utf8_lossy(&inspection.stderr)
    );
    assert_eq!(fixture.input_tree()?, input_tree, "materialization changed source or staged inputs");
    Ok(())
}

#[test]
fn public_materializer_rejects_identity_and_immutable_runtime_mutations()
-> Result<(), Box<dyn std::error::Error>> {
    for (label, source, run) in [
        ("candidate source identity mismatch", "c000000000000000000000000000000000000000", "42"),
        ("candidate run identity mismatch", "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", "43"),
    ] {
        let fixture = FinalArchiveFixture::new()?;
        reject_public(&fixture, label, fixture.materialize_public_with(&fixture.public_archive, source, run, None)?);
    }
    for (label, path) in [
        ("runtime digest mismatch", "runtime/codexy-mcp-lsp-darwin-arm64.bin"),
        ("Windows entrypoint mismatch", "mcp/codexy-mcp-codegraph.exe"),
        ("forbidden runtime-candidate.json", "runtime-candidate.json"),
        ("forbidden runtime-release.json", "runtime-release.json"),
    ] {
        let fixture = FinalArchiveFixture::new()?;
        replace_public_entry(&fixture, path)?;
        reject_public(&fixture, label, fixture.materialize_public()?);
    }
    let fixture = FinalArchiveFixture::new()?;
    reject_public(&fixture, "protected runtime mutation during current-source overlay", fixture.materialize_public_with(&fixture.public_archive, "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", "42", Some(overlay_mutator(&fixture)?))?);
    Ok(())
}

fn reject_public(fixture: &FinalArchiveFixture, label: &str, output: std::process::Output) {
    assert!(!output.status.success(), "{label} unexpectedly materialized: {output:?}");
    assert!(!fixture.final_archive.exists(), "{label} produced an accepted final archive");
}

fn replace_public_entry(fixture: &FinalArchiveFixture, relative: &str) -> Result<(), Box<dyn std::error::Error>> {
    let root = fixture.root.join("public");
    let path = root.join("plugins/codexy-devtools").join(relative);
    fs::create_dir_all(path.parent().ok_or("public mutation parent")?)?;
    fs::write(path, b"mutated public archive\n")?;
    assert!(Command::new("tar").env("COPYFILE_DISABLE", "1").args(["-C"]).arg(root).args(["-czf"]).arg(&fixture.public_archive).arg("plugins/codexy-devtools").status()?.success());
    Ok(())
}

fn overlay_mutator(fixture: &FinalArchiveFixture) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    let bin = fixture.root.join("overlay mutator");
    fs::create_dir(&bin)?;
    let python = bin.join("python3");
    fs::write(&python, "#!/bin/sh\nset -eu\n/usr/bin/python3 \"$@\"\nstate=\"$STAGED_PLUGIN/.overlay-count\"\ncount=0; test -f \"$state\" && count=$(cat \"$state\")\ncount=$((count + 1)); printf '%s\\n' \"$count\" > \"$state\"\nif test \"$count\" = 2; then printf mutated > \"$STAGED_PLUGIN/runtime/codexy-mcp-lsp-darwin-arm64.bin\"; fi\n")?;
    support::make_executable(&python)?;
    Ok(bin)
}
