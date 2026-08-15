use std::path::Path;

use anyhow::{Context as _, Result, bail};
use serde_json::Value;

use crate::paths::display_relative;

const BUNDLE: &str = "dist/codexy-marketplace-bundle.tar.gz";
const COMPONENT_MANIFEST: &str =
    "packages/getcodexy/src/codexy_runtime_tools/component-manifest.json";

pub(super) fn check(contract: &Value, path: &Path) -> Result<()> {
    let archive = contract
        .get("releaseArchive")
        .and_then(Value::as_object)
        .with_context(|| {
            format!(
                "{} releaseArchive must be an object",
                display_relative(path)
            )
        })?;
    exact(archive.get("bundle"), "releaseArchive.bundle", path, BUNDLE)?;
    exact(
        archive.get("componentManifest"),
        "releaseArchive.componentManifest",
        path,
        COMPONENT_MANIFEST,
    )
}

fn exact(value: Option<&Value>, field: &str, path: &Path, expected: &str) -> Result<()> {
    let actual = value
        .and_then(Value::as_str)
        .with_context(|| format!("{} {field} must be a string", display_relative(path)))?;
    if actual == expected {
        Ok(())
    } else {
        bail!(
            "{} {field} must be {expected:?}, got {actual:?}",
            display_relative(path)
        );
    }
}
