use std::path::Path;

use anyhow::Result;

mod capture;
mod classification;
mod economics;
mod economics_capture;
mod economics_package;
mod finding_disposition;
mod history;
mod history_contract;
mod history_evidence;
mod issue_contract;
mod ledger;
mod packet;
mod policy;
mod presence;
mod repository;
mod terminal;

pub(super) fn check(plugin_root: &Path) -> Vec<String> {
    policy::load(plugin_root)
        .and_then(|_| classification::check(plugin_root))
        .map_or_else(|error| vec![error.to_string()], |_| Vec::new())
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

pub(super) fn capture_economics(
    plugin_root: &Path,
    repository_root: &Path,
    observer_command: &Path,
    trusted_receipt: &Path,
    output: &Path,
) -> Result<()> {
    economics_capture::capture(
        plugin_root,
        repository_root,
        observer_command,
        trusted_receipt,
        output,
    )
}

pub(super) fn check_handoff(plugin_root: &Path, pr_state: &serde_json::Value) -> Vec<String> {
    terminal::check_handoff(plugin_root, pr_state)
}

pub(super) fn build_pr_state(
    plugin_root: &Path,
    base: &str,
    control: &str,
) -> Result<serde_json::Value> {
    capture::build_pr_state(plugin_root, base, control)
}

pub(super) fn is_lifecycle_terminal(plugin_root: &Path, record: &str) -> bool {
    terminal::is_lifecycle_terminal(plugin_root, record)
}
