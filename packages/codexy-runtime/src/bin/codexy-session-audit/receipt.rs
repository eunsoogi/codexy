use std::{
    collections::BTreeMap,
    path::{Component, Path},
};

use anyhow::{Context as _, Result, bail};
use serde::Serialize;

#[path = "receipt/schema.rs"]
mod schema;
use schema::*;
#[path = "receipt/input.rs"]
mod input;
#[path = "receipt/owner_tree.rs"]
mod owner_tree;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Validation {
    valid: bool,
    owner_thread_id: String,
    window_kind: &'static str,
    session_count: u64,
}

pub(super) fn validate_file(path: &Path) -> Result<Validation> {
    let bytes = input::read(path)?;
    let receipt: Receipt = serde_json::from_slice(&bytes)
        .with_context(|| format!("decoding receipt {}", path.display()))?;
    validate(receipt)
}

fn validate(receipt: Receipt) -> Result<Validation> {
    if !receipt.metadata_only || !receipt.audit.observational_only {
        bail!("receipt must be metadata-only and observational-only");
    }
    if receipt.schema_version != 1 || receipt.goal_plan_receipts.is_empty() {
        bail!("receipt must include schema version 1 and goal/plan receipts");
    }
    if receipt.metrics.event_ids.is_empty() || receipt.command_receipts.is_empty() {
        bail!("receipt must include event identifiers and command exits");
    }
    validate_digest(&receipt.audit.input_sha256)?;
    validate_installation(&receipt.installed)?;
    validate_private_metadata(&receipt.installed, &receipt.command_receipts)?;
    let boundary = &receipt.audit.comparison.owner_boundary;
    if boundary.kind != "same-owner-tree"
        || boundary.owner_thread_id != receipt.lane.owner_thread_id
    {
        bail!("receipt owner boundary must match lane owner");
    }
    validate_digest(&receipt.audit.comparison.before.input_sha256)?;
    validate_digest(&receipt.audit.comparison.after.input_sha256)?;
    let window_kind = validate_window(&receipt.audit.comparison)?;
    let totals = aggregate_sessions(
        &receipt.audit.owner_tree_sessions,
        &receipt.lane.owner_thread_id,
    )?;
    if totals != receipt.audit.owner_tree_totals {
        bail!("owner-tree totals do not match session aggregates");
    }
    validate_observations(
        &receipt.audit.comparison,
        &receipt.audit.owner_tree_sessions,
    )?;
    Ok(Validation {
        valid: true,
        owner_thread_id: receipt.lane.owner_thread_id,
        window_kind,
        session_count: totals.session_count,
    })
}

fn validate_private_metadata(installed: &Installed, commands: &[CommandReceipt]) -> Result<()> {
    if !safe_relative_path(&installed.cache_root_relative) {
        bail!("installed cache root must be a relative cache root without parent traversal");
    }
    for command in commands {
        if command.command.is_some()
            || !command.arguments_redacted
            || command.command_id.is_empty()
            || !command
                .command_id
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
        {
            bail!("command receipt must use a safe commandId and redacted arguments");
        }
    }
    Ok(())
}

fn validate_observations(comparison: &Comparison, sessions: &[OwnerSession]) -> Result<()> {
    if comparison.before.session_id == comparison.after.session_id {
        bail!("comparison observations must name distinct owner-tree sessions");
    }
    for observation in [&comparison.before, &comparison.after] {
        let session = sessions
            .iter()
            .find(|session| session.session_id == observation.session_id)
            .ok_or_else(|| {
                anyhow::anyhow!("comparison observation must name an owner-tree session")
            })?;
        if session.input_sha256 != observation.input_sha256
            || session.records_observed != observation.window.records_observed
            || session.turn_events != observation.window.turn_events
            || session.cumulative_tokens != observation.latest_cumulative_tokens
        {
            bail!("comparison observation must match its owner-tree session");
        }
    }
    Ok(())
}

fn validate_installation(installed: &Installed) -> Result<()> {
    if installed.plugin_id.is_empty() || installed.version.is_empty() {
        bail!("receipt must include installation identity");
    }
    validate_digest(&installed.manifest_sha256)?;
    let changed = changed_file_map(&installed.changed_files)?;
    let source = changed_file_map(&installed.content_proof.source_changed_files)?;
    let installed_files = changed_file_map(&installed.content_proof.installed_changed_files)?;
    validate_digest(&installed.content_proof.source_manifest_sha256)?;
    validate_digest(&installed.content_proof.installed_manifest_sha256)?;
    if !installed.content_equivalent
        || installed.content_proof.source_manifest_sha256 != installed.manifest_sha256
        || installed.content_proof.installed_manifest_sha256 != installed.manifest_sha256
        || source != changed
        || installed_files != changed
    {
        bail!(
            "receipt installed content equivalence proof must match source and installed metadata"
        );
    }
    Ok(())
}

fn changed_file_map(files: &[ChangedFile]) -> Result<BTreeMap<&str, &str>> {
    let mut mapped = BTreeMap::new();
    for changed in files {
        let path = safe_packaged_path(&changed.path)?;
        if mapped.insert(path, changed.sha256.as_str()).is_some() {
            bail!("receipt changed-file paths must be unique");
        }
        validate_digest(&changed.sha256)?;
    }
    Ok(mapped)
}

fn safe_packaged_path(path: &str) -> Result<&str> {
    let candidate = Path::new(path);
    let mut normalized = String::new();
    for component in candidate.components() {
        let Component::Normal(part) = component else {
            bail!("receipt changed-file paths must be safe repository-relative paths");
        };
        let part = part
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("receipt changed-file paths must be UTF-8"))?;
        if !normalized.is_empty() {
            normalized.push('/');
        }
        normalized.push_str(part);
    }
    if normalized.is_empty() || normalized != path || path.contains(['\\', ':']) {
        bail!("receipt changed-file paths must be safe repository-relative paths");
    }
    Ok(path)
}

fn safe_relative_path(path: &str) -> bool {
    !path.is_empty()
        && !path.contains('\\')
        && !path.contains(':')
        && Path::new(path)
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
}

fn validate_digest(value: &str) -> Result<()> {
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        bail!("receipt digest must be a 64-character SHA-256 value");
    }
    Ok(())
}

fn validate_window(comparison: &Comparison) -> Result<&'static str> {
    match comparison.window_policy {
        WindowPolicy::EqualRecordCount {
            before,
            after,
            comparable,
        } => {
            if comparison.before.window.duration_seconds.is_some()
                || comparison.after.window.duration_seconds.is_some()
                || !comparable
                || before == 0
                || before != after
                || before != comparison.before.window.records_observed
                || after != comparison.after.window.records_observed
            {
                bail!(
                    "equal record-count window must be positive, equal, observed, and comparable"
                );
            }
            Ok("equal-record-count")
        }
        WindowPolicy::EqualDuration {
            before,
            after,
            comparable,
        } => {
            if !comparable
                || before == 0
                || before != after
                || comparison.before.window.duration_seconds != Some(before)
                || comparison.after.window.duration_seconds != Some(after)
            {
                bail!("equal-duration window must be positive, equal, observed, and comparable");
            }
            Ok("equal-duration")
        }
    }
}

use owner_tree::aggregate_sessions;
