mod model;
mod resolve;
mod validation;

use anyhow::Result;
use serde_json::Value;
use std::path::Path;

pub(super) fn check(_plugin_root: &Path) -> Vec<String> {
    Vec::new()
}

pub(super) fn resolve(plugin_root: &Path, request: &str) -> Result<Value> {
    resolve::resolve(plugin_root, request)
}
