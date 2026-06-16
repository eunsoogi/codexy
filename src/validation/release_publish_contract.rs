use std::path::Path;

use anyhow::{Context as _, Result, bail};
use serde_json::Value;

use crate::paths::display_relative;
use crate::validation::{json_array_strings, load_json, require_string};

const CONTRACT_PATH: &str = ".agents/plugins/release-publish-contract.json";
const CONTRACT_SCHEMA: &str = "codexy.internal.release-publish-contract.v1";
const WORKFLOW_PATH: &str = ".github/workflows/plugin-runtime-binaries.yml";
const CURRENT_INSTALL_REF: &str = "main";
const MARKETPLACE_PATH: &str = ".agents/plugins/marketplace.json";
const PLUGIN_PATH: &str = "./plugins/codexy";
const PACKAGE_ARCHIVE: &str = "dist/codexy-marketplace-plugin.tar.gz";
const MAIN_DOGFOOD_RELEASE: &str = "codexy-main";
const FUTURE_INSTALL_REF: &str = "version-tags";

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
    check_current_marketplace_target(&contract, &contract_path)?;
    check_package_contract(&contract, &contract_path, platforms)?;
    check_source_marketplace_mode(&contract, &contract_path)?;
    check_workflow_packages_release_artifacts(&repo_root.join(WORKFLOW_PATH))
}

fn check_current_marketplace_target(contract: &Value, path: &Path) -> Result<()> {
    let snapshot = contract
        .get("currentMarketplace")
        .and_then(Value::as_object)
        .with_context(|| {
            format!(
                "{} currentMarketplace must be an object",
                display_relative(path)
            )
        })?;
    require_exact(
        snapshot.get("ref"),
        "currentMarketplace.ref",
        path,
        CURRENT_INSTALL_REF,
    )?;
    require_exact(
        snapshot.get("marketplacePath"),
        "currentMarketplace.marketplacePath",
        path,
        MARKETPLACE_PATH,
    )?;
    require_exact(
        snapshot.get("pluginPath"),
        "currentMarketplace.pluginPath",
        path,
        PLUGIN_PATH,
    )?;
    require_exact(
        snapshot.get("installCommand"),
        "currentMarketplace.installCommand",
        path,
        "codex plugin marketplace add eunsoogi/codexy --ref main",
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
    require_exact(
        package.get("mainDogfoodRelease"),
        "package.mainDogfoodRelease",
        path,
        MAIN_DOGFOOD_RELEASE,
    )?;
    require_exact(
        package.get("futureInstallRef"),
        "package.futureInstallRef",
        path,
        FUTURE_INSTALL_REF,
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

fn check_workflow_packages_release_artifacts(path: &Path) -> Result<()> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("reading {}", display_relative(path)))?;
    for required in [
        "Assemble marketplace plugin package",
        "dist/codexy-marketplace-plugin",
        "dist/codexy-marketplace-plugin.tar.gz",
        "scripts/validate-plugin-config --plugin-root \"$plugin_root\" --check-runtime-artifacts",
        "gh release upload",
        "codexy-main",
        "concurrency:",
        "cancel-in-progress: true",
        "Verify main dogfood package is current",
        "repos/${GITHUB_REPOSITORY}/git/ref/heads/main",
        "steps.main-package-head.outputs.current == 'true'",
        "Create main dogfood package release",
        "Upload main dogfood package",
    ] {
        if !text.contains(required) {
            bail!(
                "{} must package Codexy release artifacts; missing {required:?}",
                display_relative(path)
            );
        }
    }
    for forbidden in [
        "Publish generated marketplace snapshot",
        "MARKETPLACE_BRANCH",
        "dist/marketplace-root",
        "git -C \"$marketplace_root\" push --force origin \"$MARKETPLACE_BRANCH\"",
    ] {
        if text.contains(forbidden) {
            bail!(
                "{} must not require a generated marketplace branch for current dogfood installs; found {forbidden:?}",
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
