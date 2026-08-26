use std::{fs, path::{Path, PathBuf}};

use serde_json::{Value, json};

use crate::support::{self, FixtureCommand as Command};

use super::final_archive_fixture::FinalArchiveFixture;

const COMPONENT_MANIFEST: &str =
    "packages/getcodexy/src/codexy_runtime_tools/component-manifest.json";
const MARKETPLACE: &str = ".agents/plugins/marketplace.json";
const PLUGIN_MANIFESTS: [&str; 3] = [
    "plugins/codexy/.codex-plugin/plugin.json",
    "plugins/codexy-github/.codex-plugin/plugin.json",
    "plugins/codexy-devtools/.codex-plugin/plugin.json",
];

#[test]
fn release_train_assembler_emits_a_reproducible_complete_bundle()
    -> Result<(), Box<dyn std::error::Error>> {
    let fixture = FinalArchiveFixture::new()?;
    let root = codexy_runtime::paths::repository_root();
    let candidate_version = component_version(root)?;
    let release_tag = format!("v{candidate_version}");
    set_manifest_version(&fixture.root.join(PLUGIN_MANIFESTS[2]), &candidate_version)?;
    fs::copy(root.join("plugins/codexy-devtools/.mcp.json"), fixture.root.join("plugins/codexy-devtools/.mcp.json"))?;
    assert!(fixture.materialize_public_for_tag(&release_tag)?.status.success());
    for relative in [
        "plugins/codexy",
        "plugins/codexy-github",
        COMPONENT_MANIFEST,
        MARKETPLACE,
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
    project_release_versions(&fixture.root, &candidate_version)?;
    let assembler = fixture.root.join("scripts/assemble-release-train-archive.sh");
    fs::create_dir_all(assembler.parent().ok_or("assembler parent")?)?;
    fs::copy(root.join("scripts/assemble-release-train-archive.sh"), &assembler)?;
    support::make_executable(&assembler)?;
    let first = fixture.root.join("bundle-one.tar.gz");
    let second = fixture.root.join("bundle-two.tar.gz");
    for output in [&first, &second] {
        let result = Command::new(&assembler).arg_path(&fixture.final_archive).arg_path(output).current_dir(&fixture.root).env("RELEASE_TAG", &release_tag).output()?;
        assert!(result.status.success(), "{}", String::from_utf8_lossy(&result.stderr));
    }
    assert_eq!(fs::read(&first)?, fs::read(&second)?);
    Ok(())
}

#[test]
fn release_train_inspector_accepts_the_complete_activation_checkout()
-> Result<(), Box<dyn std::error::Error>> {
    let temporary = tempfile::tempdir()?;
    let root = codexy_runtime::paths::repository_root();
    let candidate_version = component_version(root)?;
    let release_tag = format!("v{candidate_version}");
    let checkout = release_checkout(root, temporary.path(), &candidate_version)?;
    let runtime = temporary.path().join("runtime.tar.gz");
    let bundle = temporary.path().join("bundle.tar.gz");
    let staged = temporary.path().join("staged/plugins/codexy-devtools");
    support::copy_dir(&checkout.join("plugins/codexy-devtools"), &staged)?;
    materialize_core_handoff_fixture(&checkout, &staged)?;
    for contract in ["runtime-candidate.json", "runtime-release.json"] {
        let path = staged.join(contract);
        if path.exists() {
            fs::remove_file(path)?;
        }
    }
    let manifest_path = staged.join(".codex-plugin/plugin.json");
    let mut manifest: serde_json::Value = serde_json::from_slice(&fs::read(&manifest_path)?)?;
    manifest["supportedPlatforms"] = serde_json::json!(["darwin-arm64", "linux-x86_64", "windows-x86_64"]);
    fs::write(&manifest_path, format!("{}\n", serde_json::to_string_pretty(&manifest)?))?;
    let wrapper_path = staged.join("mcp/codexy-mcp-devtools");
    let wrapper = fs::read_to_string(&wrapper_path)?.replace("bundled_platforms=\"darwin-arm64 linux-x86_64\"", "bundled_platforms=\"darwin-arm64 linux-x86_64 windows-x86_64\"");
    let version_pattern = regex::Regex::new(r"(exec uvx --from getcodexy==)[0-9]+\.[0-9]+\.[0-9]+")?;
    let wrapper = version_pattern.replace(&wrapper, format!("${{1}}{candidate_version}")).into_owned();
    fs::write(wrapper_path, wrapper)?;
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
    let assembled = Command::new(checkout.join("scripts/assemble-release-train-archive.sh"))
        .arg_path(&runtime)
        .arg_path(&bundle)
        .current_dir(&checkout)
        .env("RELEASE_TAG", &release_tag)
        .output()?;
    assert!(assembled.status.success(), "{}", String::from_utf8_lossy(&assembled.stderr));
    let inspected = Command::new(root.join("scripts/inspect_release_train_archive.py"))
        .arg_path(&bundle)
        .arg_path(&checkout)
        .arg(&release_tag)
        .output()?;
    assert!(inspected.status.success(), "{}", String::from_utf8_lossy(&inspected.stderr));
    let staging_receipt = temporary.path().join("staging-receipt.json");
    let staging_run = temporary.path().join("staging-run.json");
    let runtime = temporary.path().join("runtime.tar.gz");
    let receipt = temporary.path().join("release-receipt.json");
    fs::write(&staging_receipt, r#"{"provenance":{"runId":42}}"#)?;
    fs::write(&staging_run, r#"{"run_attempt":7}"#)?;
    let created = Command::new(checkout.join("scripts/create_release_train_receipt.py"))
        .arg_path(&runtime)
        .arg_path(&bundle)
        .arg_path(&runtime)
        .arg_path(&staging_receipt)
        .arg_path(&staging_run)
        .arg_path(&receipt)
        .current_dir(&checkout)
        .env("RELEASE_TAG", &release_tag)
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
    let assembled = Command::new(checkout.join("scripts/assemble-release-train-archive.sh"))
        .arg_path(&tampered_runtime)
        .arg_path(&tampered_bundle)
        .current_dir(&checkout)
        .env("RELEASE_TAG", &release_tag)
        .output()?;
    assert!(assembled.status.success());
    let rejected = Command::new(root.join("scripts/inspect_release_train_archive.py"))
        .arg_path(&tampered_bundle)
        .arg_path(&checkout)
        .arg(&release_tag)
        .output()?;
    assert!(!rejected.status.success(), "extra bundle file was accepted");
    Ok(())
}

fn materialize_core_handoff_fixture(checkout: &Path, staged: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let activation_path = checkout.join(".agents/plugins/runtime-activation.json");
    let mut activation: Value = serde_json::from_slice(&fs::read(&activation_path)?)?;
    let mut platforms = serde_json::Map::new();
    fs::create_dir_all(staged.join("runtime"))?;
    for (platform, extension, kind) in [("darwin-arm64", "bin", "mach-o"), ("linux-x86_64", "bin", "elf"), ("windows-x86_64", "exe", "pe")] {
        let relative = format!("runtime/codexy-handoff-validate-{platform}.{extension}");
        let path = staged.join(&relative);
        fs::write(&path, format!("fixture core handoff {platform}\n"))?;
        if extension == "bin" { support::make_executable(&path)?; }
        platforms.insert(platform.to_owned(), json!({"path": relative, "sha256": support::sha256_file(&path)?, "kind": kind}));
    }
    let handoff = json!({"schema": "codexy.handoff-runtime.v1", "version": 1, "source": {"commit": activation["candidate"]["source"]["commit"], "tree": activation["candidate"]["source"]["tree"]}, "platforms": platforms});
    let handoff_path = staged.join("handoff-runtime.json");
    fs::write(&handoff_path, serde_json::to_vec(&handoff)?)?;
    activation["candidate"]["classes"]["coreHandoff"] = json!({"manifest": {"path": "handoff-runtime.json", "sha256": support::sha256_file(&handoff_path)?}, "platforms": handoff["platforms"]});
    fs::write(activation_path, format!("{}\n", serde_json::to_string_pretty(&activation)?))?;
    Ok(())
}

fn component_version(root: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let manifest: serde_json::Value = serde_json::from_slice(&fs::read(root.join(COMPONENT_MANIFEST))?)?;
    manifest["components"]
        .as_array()
        .and_then(|components| components.first())
        .and_then(|component| component["version"].as_str())
        .map(ToOwned::to_owned)
        .ok_or_else(|| "component manifest version".into())
}

fn set_manifest_version(path: &Path, version: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut manifest: serde_json::Value = serde_json::from_slice(&fs::read(path)?)?;
    manifest["version"] = serde_json::Value::String(version.to_owned());
    fs::write(path, format!("{}\n", serde_json::to_string_pretty(&manifest)?))?;
    Ok(())
}

fn project_release_versions(root: &Path, version: &str) -> Result<(), Box<dyn std::error::Error>> {
    for relative in PLUGIN_MANIFESTS {
        set_manifest_version(&root.join(relative), version)?;
    }
    let path = root.join(MARKETPLACE);
    let mut marketplace: serde_json::Value = serde_json::from_slice(&fs::read(&path)?)?;
    for plugin in marketplace["plugins"]
        .as_array_mut()
        .ok_or("marketplace plugins")?
    {
        plugin["version"] = serde_json::Value::String(version.to_owned());
    }
    fs::write(path, format!("{}\n", serde_json::to_string_pretty(&marketplace)?))?;
    Ok(())
}

fn release_checkout(
    root: &Path,
    parent: &Path,
    version: &str,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let checkout = parent.join("activation-checkout");
    for relative in ["plugins/codexy", "plugins/codexy-github", "plugins/codexy-devtools"] {
        support::copy_dir(root.join(relative), &checkout.join(relative))?;
    }
    for relative in [COMPONENT_MANIFEST, MARKETPLACE, ".agents/plugins/runtime-activation.json"] {
        let target = checkout.join(relative);
        fs::create_dir_all(target.parent().ok_or("checkout artifact parent")?)?;
        fs::copy(root.join(relative), target)?;
    }
    fs::create_dir_all(checkout.join("scripts"))?;
    for script in [
        "assemble-release-train-archive.sh",
        "create_release_train_receipt.py",
        "handoff_runtime_contract.py",
    ] {
        let target = checkout.join("scripts").join(script);
        fs::copy(root.join("scripts").join(script), &target)?;
        if script.ends_with(".sh") {
            support::make_executable(&target)?;
        }
    }
    project_release_versions(&checkout, version)?;
    Ok(checkout)
}
