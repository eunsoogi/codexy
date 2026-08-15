use std::fs;

use crate::support::{self, FixtureCommand as Command};

use super::final_archive_fixture::FinalArchiveFixture;

#[test]
fn release_train_assembler_emits_a_reproducible_complete_bundle()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = FinalArchiveFixture::new()?;
    let root = codexy_runtime::paths::repository_root();
    fs::copy(
        root.join("plugins/codexy-devtools/.mcp.json"),
        fixture.root.join("plugins/codexy-devtools/.mcp.json"),
    )?;
    assert!(fixture.materialize_public()?.status.success());
    for relative in [
        "plugins/codexy",
        "plugins/codexy-github",
        "packages/getcodexy/src/codexy_runtime_tools/component-manifest.json",
        ".agents/plugins/marketplace.json",
    ] {
        let source = root.join(relative);
        let target = fixture.root.join(relative);
        if source.is_dir() {
            support::copy_dir(&source, &target)?;
        } else {
            fs::create_dir_all(target.parent().ok_or("bundle fixture parent")?)?;
            fs::copy(source, target)?;
        }
    }
    let first = fixture.root.join("bundle-one.tar.gz");
    let second = fixture.root.join("bundle-two.tar.gz");
    for output in [&first, &second] {
        let result = Command::new(root.join("scripts/assemble-release-train-archive.sh"))
            .arg_path(&fixture.final_archive)
            .arg_path(output)
            .current_dir(&fixture.root)
            .env("RELEASE_TAG", "v1.3.0")
            .output()?;
        assert!(result.status.success(), "{}", String::from_utf8_lossy(&result.stderr));
    }
    assert_eq!(fs::read(&first)?, fs::read(&second)?);
    let listing = Command::new("tar").args(["-tzf"]).arg_path(&first).output()?;
    assert!(listing.status.success());
    let listing = String::from_utf8(listing.stdout)?;
    for path in [
        ".agents/plugins/marketplace.json",
        "plugins/codexy/.codex-plugin/plugin.json",
        "plugins/codexy-github/.codex-plugin/plugin.json",
        "plugins/codexy-devtools/.codex-plugin/plugin.json",
    ] {
        assert!(listing.lines().any(|entry| entry == path), "missing {path}");
    }
    Ok(())
}

#[test]
fn release_train_inspector_accepts_the_complete_activation_checkout()
-> Result<(), Box<dyn std::error::Error>> {
    let temporary = tempfile::tempdir()?;
    let root = codexy_runtime::paths::repository_root();
    let runtime = temporary.path().join("runtime.tar.gz");
    let bundle = temporary.path().join("bundle.tar.gz");
    let staged = temporary.path().join("staged/plugins/codexy-devtools");
    support::copy_dir(&root.join("plugins/codexy-devtools"), &staged)?;
    for contract in ["runtime-candidate.json", "runtime-release.json"] {
        let path = staged.join(contract);
        if path.exists() {
            fs::remove_file(path)?;
        }
    }
    let manifest_path = staged.join(".codex-plugin/plugin.json");
    let mut manifest: serde_json::Value = serde_json::from_slice(&fs::read(&manifest_path)?)?;
    manifest["supportedPlatforms"] = serde_json::json!([
        "darwin-arm64", "linux-x86_64", "windows-x86_64"
    ]);
    fs::write(&manifest_path, format!("{}\n", serde_json::to_string_pretty(&manifest)?))?;
    fs::create_dir_all(staged.join("runtime"))?;
    fs::create_dir_all(staged.join("mcp"))?;
    fs::write(staged.join("mcp/codexy-mcp-devtools.exe"), "dispatcher\n")?;
    for platform in ["darwin-arm64", "linux-x86_64", "windows-x86_64"] {
        let extension = if platform == "windows-x86_64" { "exe" } else { "bin" };
        for server in ["lsp", "codegraph"] {
            fs::write(
                staged.join(format!("runtime/codexy-mcp-{server}-{platform}.{extension}")),
                format!("{server}-{platform}\n"),
            )?;
        }
    }
    assert!(Command::new("tar")
        .args(["-C"])
        .arg_path(temporary.path().join("staged"))
        .args(["-czf"])
        .arg_path(&runtime)
        .arg("plugins/codexy-devtools")
        .status()?
        .success());
    let assembled = Command::new(root.join("scripts/assemble-release-train-archive.sh"))
        .arg_path(&runtime)
        .arg_path(&bundle)
        .current_dir(&root)
        .env("RELEASE_TAG", "v1.3.0")
        .output()?;
    assert!(assembled.status.success(), "{}", String::from_utf8_lossy(&assembled.stderr));
    let inspected = Command::new(root.join("scripts/inspect_release_train_archive.py"))
        .arg_path(&bundle)
        .arg_path(&root)
        .arg("v1.3.0")
        .output()?;
    assert!(inspected.status.success(), "{}", String::from_utf8_lossy(&inspected.stderr));
    let staging_receipt = temporary.path().join("staging-receipt.json");
    let staging_run = temporary.path().join("staging-run.json");
    let runtime = temporary.path().join("runtime.tar.gz");
    let receipt = temporary.path().join("release-receipt.json");
    fs::write(&staging_receipt, r#"{"provenance":{"runId":42}}"#)?;
    fs::write(&staging_run, r#"{"run_attempt":7}"#)?;
    let created = Command::new(root.join("scripts/create_release_train_receipt.py"))
        .arg_path(&runtime)
        .arg_path(&bundle)
        .arg_path(&runtime)
        .arg_path(&staging_receipt)
        .arg_path(&staging_run)
        .arg_path(&receipt)
        .current_dir(&root)
        .env("RELEASE_TAG", "v1.3.0")
        .env("ACTIVATION_COMMIT", "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb")
        .env("STAGING_SOURCE_COMMIT", "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
        .env("STAGING_RUN_ID", "42")
        .output()?;
    assert!(created.status.success(), "{}", String::from_utf8_lossy(&created.stderr));
    let receipt: serde_json::Value = serde_json::from_slice(&fs::read(receipt)?)?;
    assert_eq!(receipt["schema"], "codexy-runtime-release-receipt/v2");
    assert_eq!(receipt["components"].as_array().ok_or("receipt components")?.len(), 3);
    fs::write(staged.join("unexpected.txt"), "unexpected\n")?;
    let tampered_runtime = temporary.path().join("tampered-runtime.tar.gz");
    assert!(Command::new("tar")
        .args(["-C"])
        .arg_path(temporary.path().join("staged"))
        .args(["-czf"])
        .arg_path(&tampered_runtime)
        .arg("plugins/codexy-devtools")
        .status()?
        .success());
    let tampered_bundle = temporary.path().join("tampered-bundle.tar.gz");
    let assembled = Command::new(root.join("scripts/assemble-release-train-archive.sh"))
        .arg_path(&tampered_runtime)
        .arg_path(&tampered_bundle)
        .current_dir(&root)
        .env("RELEASE_TAG", "v1.3.0")
        .output()?;
    assert!(assembled.status.success());
    let rejected = Command::new(root.join("scripts/inspect_release_train_archive.py"))
        .arg_path(&tampered_bundle)
        .arg_path(&root)
        .arg("v1.3.0")
        .output()?;
    assert!(!rejected.status.success(), "extra bundle file was accepted");
    Ok(())
}
