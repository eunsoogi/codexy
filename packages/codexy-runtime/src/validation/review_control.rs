use std::path::Path;

use anyhow::Result;

mod economics;
mod ledger;
mod packet;
mod policy;
mod repository;

pub(super) fn check(plugin_root: &Path) -> Vec<String> {
    policy::load(plugin_root)
        .map(|_| Vec::new())
        .unwrap_or_else(|error| vec![error.to_string()])
}

pub(super) fn resolve_profile(plugin_root: &Path, request: &str) -> Result<serde_json::Value> {
    policy::resolve(plugin_root, request)
}

pub(super) fn check_packet(
    plugin_root: &Path,
    repository_root: &Path,
    ledger_path: &Path,
    packet: &str,
) -> Result<()> {
    packet::check(plugin_root, repository_root, ledger_path, packet)
}

pub(super) fn check_economics(
    plugin_root: &Path,
    repository_root: &Path,
    economics: &str,
) -> Result<()> {
    economics::check(plugin_root, repository_root, economics)
}
