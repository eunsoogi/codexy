use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Component, Path},
};

use anyhow::{Context as _, Result, bail};
use serde::Serialize;

use super::audit_math::checked_add;

#[path = "receipt/schema.rs"]
mod schema;
use schema::*;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Validation {
    valid: bool,
    owner_thread_id: String,
    window_kind: &'static str,
    session_count: u64,
}

pub(super) fn validate_file(path: &Path) -> Result<Validation> {
    let bytes = fs::read(path).with_context(|| format!("reading receipt {}", path.display()))?;
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
    Ok(Validation {
        valid: true,
        owner_thread_id: receipt.lane.owner_thread_id,
        window_kind,
        session_count: totals.session_count,
    })
}

fn validate_private_metadata(installed: &Installed, commands: &[CommandReceipt]) -> Result<()> {
    let cache_root = Path::new(&installed.cache_root_relative);
    if installed.cache_root_relative.is_empty()
        || cache_root.is_absolute()
        || cache_root
            .components()
            .any(|part| matches!(part, Component::ParentDir | Component::RootDir))
    {
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
        if !mapped.insert(path, changed.sha256.as_str()).is_none() {
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
    if normalized.is_empty() || normalized != path || path.contains('\\') {
        bail!("receipt changed-file paths must be safe repository-relative paths");
    }
    Ok(path)
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

fn aggregate_sessions(sessions: &[OwnerSession], owner: &str) -> Result<Totals> {
    if sessions.is_empty() {
        bail!("owner tree must contain at least one session");
    }
    let mut ids = BTreeSet::new();
    let mut totals = Totals::default();
    for session in sessions {
        if session.owner_root_thread_id != owner {
            bail!("owner-tree session does not match owner boundary");
        }
        if !ids.insert(&session.session_id) {
            bail!("owner tree contains a duplicate session");
        }
        totals.session_count = checked_add(totals.session_count, 1, "owner-tree session count")?;
        add(
            &mut totals.records_observed,
            session.records_observed,
            "records",
        )?;
        add(&mut totals.turn_events, session.turn_events, "turns")?;
        add(
            &mut totals.cumulative_tokens,
            session.cumulative_tokens,
            "tokens",
        )?;
        add(
            &mut totals.tool_input_bytes,
            session.tool_input_bytes,
            "tool input",
        )?;
        add(
            &mut totals.tool_output_bytes,
            session.tool_output_bytes,
            "tool output",
        )?;
        add_family(&mut totals.exec_family, &session.exec_family)?;
        add_family(&mut totals.wait_family, &session.wait_family)?;
    }
    Ok(totals)
}

fn add(target: &mut u64, value: u64, label: &str) -> Result<()> {
    *target = checked_add(*target, value, &format!("owner-tree {label}"))?;
    Ok(())
}

fn add_family(target: &mut Family, value: &Family) -> Result<()> {
    add(&mut target.calls, value.calls, "family calls")?;
    add(&mut target.input_bytes, value.input_bytes, "family input")?;
    add(
        &mut target.output_bytes,
        value.output_bytes,
        "family output",
    )
}
