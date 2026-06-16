use std::path::Path;

use anyhow::{Context as _, Result, bail};
use serde_json::Value;

use crate::paths::display_relative;
use crate::validation::{json_array_strings, load_json, require_string};

const CONTRACT_PATH: &str = ".agents/plugins/release-publish-contract.json";
const CONTRACT_SCHEMA: &str = "codexy.internal.release-publish-contract.v1";
const WORKFLOW_PATH: &str = ".github/workflows/plugin-runtime-binaries.yml";
const MARKETPLACE_BRANCH: &str = "codexy-marketplace";
const MARKETPLACE_PATH: &str = ".agents/plugins/marketplace.json";
const PLUGIN_PATH: &str = "./plugins/codexy";
const PACKAGE_ARCHIVE: &str = "dist/codexy-marketplace-plugin.tar.gz";

pub(super) fn check_snapshot_contract(platforms: &[String]) -> Result<()> {
    let repo_root = crate::paths::repo_root()?;
    let contract_path = repo_root.join(CONTRACT_PATH);
    let contract = load_json(&contract_path)?;
    require_exact(
        contract.get("schema"),
        "schema",
        &contract_path,
        CONTRACT_SCHEMA,
    )?;
    require_string(contract.get("name"), "name", &contract_path)?;
    check_snapshot_target(&contract, &contract_path)?;
    check_package_contract(&contract, &contract_path, platforms)?;
    check_source_marketplace_mode(&contract, &contract_path)?;
    check_workflow_publishes_snapshot(&repo_root.join(WORKFLOW_PATH))
}

fn check_snapshot_target(contract: &Value, path: &Path) -> Result<()> {
    let snapshot = contract
        .get("marketplaceSnapshot")
        .and_then(Value::as_object)
        .with_context(|| {
            format!(
                "{} marketplaceSnapshot must be an object",
                display_relative(path)
            )
        })?;
    require_exact(
        snapshot.get("ref"),
        "marketplaceSnapshot.ref",
        path,
        MARKETPLACE_BRANCH,
    )?;
    require_exact(
        snapshot.get("marketplacePath"),
        "marketplaceSnapshot.marketplacePath",
        path,
        MARKETPLACE_PATH,
    )?;
    require_exact(
        snapshot.get("pluginPath"),
        "marketplaceSnapshot.pluginPath",
        path,
        PLUGIN_PATH,
    )?;
    require_exact(
        snapshot.get("installCommand"),
        "marketplaceSnapshot.installCommand",
        path,
        "codex plugin marketplace add eunsoogi/codexy --ref codexy-marketplace",
    )
}

fn check_package_contract(contract: &Value, path: &Path, platforms: &[String]) -> Result<()> {
    let package = contract
        .get("package")
        .and_then(Value::as_object)
        .with_context(|| format!("{} package must be an object", display_relative(path)))?;
    require_exact(
        package.get("workflow"),
        "package.workflow",
        path,
        WORKFLOW_PATH,
    )?;
    require_exact(
        package.get("archive"),
        "package.archive",
        path,
        PACKAGE_ARCHIVE,
    )?;
    let package_platforms = json_array_strings(package.get("platforms")).with_context(|| {
        format!(
            "{} package.platforms must be an array",
            display_relative(path)
        )
    })?;
    if package_platforms != platforms {
        bail!(
            "{} package.platforms must match supportedPlatforms: expected {:?}, got {:?}",
            display_relative(path),
            platforms,
            package_platforms
        );
    }
    Ok(())
}

fn check_source_marketplace_mode(contract: &Value, path: &Path) -> Result<()> {
    let source = contract
        .get("sourceMarketplace")
        .and_then(Value::as_object)
        .with_context(|| {
            format!(
                "{} sourceMarketplace must document source checkout mode",
                display_relative(path)
            )
        })?;
    require_exact(
        source.get("path"),
        "sourceMarketplace.path",
        path,
        MARKETPLACE_PATH,
    )?;
    require_exact(
        source.get("mode"),
        "sourceMarketplace.mode",
        path,
        "source-checkout-dev",
    )
}

fn check_workflow_publishes_snapshot(path: &Path) -> Result<()> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("reading {}", display_relative(path)))?;
    for required in [
        "Publish generated marketplace snapshot",
        "MARKETPLACE_BRANCH: codexy-marketplace",
        "dist/marketplace-root",
        "cp .agents/plugins/marketplace.json \"$marketplace_root/.agents/plugins/marketplace.json\"",
        "cp -R \"$PACKAGE_ROOT/plugins/codexy\" \"$marketplace_root/plugins/codexy\"",
        "scripts/validate-plugin-config --plugin-root \"$marketplace_root/plugins/codexy\" --check-runtime-artifacts",
        "git -C \"$marketplace_root\" push --force origin \"$MARKETPLACE_BRANCH\"",
    ] {
        if !text.contains(required) {
            bail!(
                "{} must publish an installable Codex marketplace snapshot; missing {required:?}",
                display_relative(path)
            );
        }
    }
    Ok(())
}

fn require_exact(value: Option<&Value>, field: &str, path: &Path, expected: &str) -> Result<()> {
    let actual = value
        .and_then(Value::as_str)
        .with_context(|| format!("{} {field} must be a string", display_relative(path)))?;
    if actual == expected {
        Ok(())
    } else {
        bail!(
            "{} {field} must be {expected:?}, got {actual:?}",
            display_relative(path)
        )
    }
}
