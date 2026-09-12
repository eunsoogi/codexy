use std::{fs, path::{Path, PathBuf}};

use crate::support;

use super::{COMPONENT_MANIFEST, MARKETPLACE, PLUGIN_MANIFESTS};

pub(super) fn copy_component_mcp(
    root: &Path,
    fixture_root: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    for plugin_root in [
        fixture_root.join("plugins/codexy-devtools"),
        fixture_root.join("staged/plugins/codexy-devtools"),
    ] {
        for relative in [".mcp.json", "mcp/codexy_mcp_bootstrap.py"] {
            fs::copy(
                root.join("plugins/codexy-devtools").join(relative),
                plugin_root.join(relative),
            )?;
        }
    }
    Ok(())
}

pub(super) fn project_release_versions(
    root: &Path,
    version: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    for relative in PLUGIN_MANIFESTS {
        super::set_manifest_version(&root.join(relative), version)?;
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

pub(super) fn release_checkout(
    root: &Path,
    parent: &Path,
    version: &str,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let checkout = parent.join("activation-checkout");
    for relative in [
        "plugins/codexy",
        "plugins/codexy-github",
        "plugins/codexy-devtools",
    ] {
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
