use std::path::Path;

use anyhow::Result;
use serde_json::Value;

mod classification;
mod external_finding;
mod history;
mod migration;
mod native_history;
mod operations;
mod policy;
mod post_cap_disposition;
mod pre_pr;
mod pre_verdict;
mod request;
mod snapshot;
mod state;
mod transition;

pub(super) fn check(plugin_root: &Path) -> Vec<String> {
    policy::load(plugin_root)
        .and_then(|_| classification::check(plugin_root))
        .map_or_else(|error| vec![error.to_string()], |_| Vec::new())
}

pub(super) fn resolve_profile(plugin_root: &Path, request: &str) -> Result<Value> {
    policy::resolve(plugin_root, request)
}

pub(super) use native_history::recover_native_history;
pub(super) use operations::{
    build_pr_state, check_next_review_eligibility, import_pre_pr_history, produce,
};

pub(super) fn check_packet(
    _plugin_root: &Path,
    _repository_root: &Path,
    _legacy_output: &Path,
    _legacy_input: &str,
) -> Result<()> {
    Ok(())
}

pub(super) fn check_economics(
    _plugin_root: &Path,
    _repository_root: &Path,
    _legacy_input: &str,
) -> Result<()> {
    Ok(())
}

pub(super) fn check_handoff(plugin_root: &Path, state: &Value) -> Vec<String> {
    let mut state = state.clone();
    if let Some(mut control) = state.get("reviewControl").cloned() {
        if external_finding::requires_source(&control) {
            if let Err(error) = external_finding::refresh_live(&mut control) {
                return vec![error];
            }
        } else if post_cap_disposition::requires_source(&control) {
            if let Err(error) = post_cap_disposition::refresh_live(&mut control, Some(&state)) {
                return vec![error];
            }
        }
        if let Some(object) = state.as_object_mut() {
            object.insert("reviewControl".into(), control);
        }
    }
    state::check_pr_state(plugin_root, &state, true)
        .err()
        .into_iter()
        .collect()
}

pub(super) fn is_lifecycle_terminal(plugin_root: &Path, record: &str) -> bool {
    state::is_lifecycle_terminal(plugin_root, record)
}
