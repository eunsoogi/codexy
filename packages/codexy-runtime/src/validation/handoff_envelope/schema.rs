use super::OmissionReason;
use anyhow::{Result, bail, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StableHandoff {
    pub policy_digest: String,
    pub workflow_profile: String,
    pub task_classification: String,
    pub selected_references: Vec<String>,
}
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IssuePrIdentity {
    pub issue: Option<u64>,
    pub pr: Option<u64>,
}
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OwnerWorktree {
    pub owner: String,
    pub branch: String,
    pub worktree: String,
}
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BaseHeadSha {
    pub base: String,
    pub head: String,
}
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DirtyIndexState {
    pub dirty: bool,
    pub index: bool,
}
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewThread {
    pub id: String,
    pub outdated: bool,
}
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct HandoffEvent {
    pub id: String,
    pub kind: String,
    pub lane: String,
    pub subject: String,
    pub delta: String,
}
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct HandoffVolatile {
    pub issue_pr_identity: IssuePrIdentity,
    pub owner_worktree: OwnerWorktree,
    pub base_head_sha: BaseHeadSha,
    pub dirty_index_state: DirtyIndexState,
    pub checks: Vec<String>,
    pub unresolved_review_threads: Vec<ReviewThread>,
    pub selected_reviewer_state: String,
    pub verification: Vec<String>,
    pub active_obligation: String,
    pub external_gate: String,
    pub next_action: String,
    pub child_task: Option<String>,
    pub parent_task: Option<String>,
    pub preserved_artifacts: Option<String>,
    pub delivery: String,
    pub task_surface: String,
    pub event: HandoffEvent,
    pub authoritative_refresh_handles: Vec<String>,
    pub omissions: BTreeMap<String, OmissionReason>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HandoffEnvelope {
    pub schema: String,
    pub stable: StableHandoff,
    pub volatile: HandoffVolatile,
    pub stable_identity: String,
    pub volatile_identity: String,
}
pub(super) fn validate_unique_strings(values: &[String], field: &str) -> Result<()> {
    ensure!(
        !values.iter().any(String::is_empty),
        "{field} contains an empty value"
    );
    ensure!(
        values.iter().collect::<BTreeSet<_>>().len() == values.len(),
        "{field} contains duplicate values"
    );
    Ok(())
}
pub(super) fn validate_volatile(value: &HandoffVolatile) -> Result<()> {
    token(&value.owner_worktree.owner, "owner")?;
    token(&value.owner_worktree.branch, "branch")?;
    token(&value.owner_worktree.worktree, "worktree")?;
    token(&value.base_head_sha.base, "base SHA")?;
    token(&value.base_head_sha.head, "head SHA")?;
    validate_tokens(&value.checks, "checks")?;
    validate_texts(&value.verification, "verification")?;
    let mut review_ids = BTreeSet::new();
    for thread in &value.unresolved_review_threads {
        token(&thread.id, "review thread id")?;
        ensure!(review_ids.insert(&thread.id), "duplicate review thread id");
    }
    text(
        &value.selected_reviewer_state,
        "selected reviewer state",
        256,
    )?;
    text(&value.active_obligation, "active obligation", 256)?;
    text(&value.external_gate, "external gate", 256)?;
    text(&value.next_action, "next action", 256)?;
    for (field, item) in [
        ("child task", value.child_task.as_deref()),
        ("parent task", value.parent_task.as_deref()),
        ("preserved artifacts", value.preserved_artifacts.as_deref()),
    ] {
        if let Some(item) = item {
            text(item, field, 256)?;
        }
    }
    validate_tokens(
        &value.authoritative_refresh_handles,
        "authoritative refresh handles",
    )?;
    validate_omissions(value)?;
    text(&value.delivery, "delivery", 256)?;
    text(&value.task_surface, "task surface", 256)?;
    validate_event(&value.event)
}
fn validate_event(value: &HandoffEvent) -> Result<()> {
    for (item, field) in [
        (&value.kind, "event kind"),
        (&value.lane, "event lane"),
        (&value.subject, "event subject"),
    ] {
        token(item, field)?;
    }
    ensure!(
        value.id == format!("{}|{}|{}", value.kind, value.lane, value.subject),
        "event id does not match event fields"
    );
    text(&value.delta, "event delta", 1024)
}
fn validate_tokens(values: &[String], field: &str) -> Result<()> {
    validate_unique_strings(values, field)?;
    values.iter().try_for_each(|value| token(value, field))
}
fn validate_texts(values: &[String], field: &str) -> Result<()> {
    validate_unique_strings(values, field)?;
    values.iter().try_for_each(|value| text(value, field, 256))
}
fn validate_omissions(value: &HandoffVolatile) -> Result<()> {
    for (field, present) in [
        ("issue", value.issue_pr_identity.issue.is_some()),
        ("pr", value.issue_pr_identity.pr.is_some()),
        ("child_task", value.child_task.is_some()),
        ("parent_task", value.parent_task.is_some()),
        ("preserved_artifacts", value.preserved_artifacts.is_some()),
        (
            "authoritative_refresh_handles",
            !value.authoritative_refresh_handles.is_empty(),
        ),
    ] {
        ensure!(
            present != value.omissions.contains_key(field),
            "omission state: {field}"
        );
    }
    ensure!(
        value.omissions.keys().all(|field| matches!(
            field.as_str(),
            "issue"
                | "pr"
                | "child_task"
                | "parent_task"
                | "preserved_artifacts"
                | "authoritative_refresh_handles"
        )),
        "unknown omission field"
    );
    Ok(())
}
pub(super) fn token(value: &str, field: &str) -> Result<()> {
    ensure!(
        !value.is_empty() && value.len() <= 256 && !value.chars().any(char::is_whitespace),
        "{field} must be a bounded non-empty token"
    );
    Ok(())
}
pub(super) fn section<'a>(input: &'a str, heading: &str) -> Result<&'a str> {
    ensure!(
        exact_heading_count(input, heading) == 1,
        "legacy section is repeated or missing"
    );
    let start = input
        .find(heading)
        .ok_or_else(|| anyhow::anyhow!("legacy section is missing"))?
        + heading.len();
    let rest = &input[start..];
    Ok(&rest[..rest.find("\n## ").unwrap_or(rest.len())])
}
pub(super) fn exact_heading_count(s: &str, h: &str) -> usize {
    s.lines().filter(|l| *l == h).count()
}
pub(super) fn required<'a>(fields: &'a BTreeMap<String, String>, key: &str) -> Result<&'a str> {
    fields
        .get(key)
        .map(String::as_str)
        .ok_or_else(|| anyhow::anyhow!("legacy field {key} is missing"))
}
pub(super) fn require_same(
    fields: &BTreeMap<String, String>,
    key: &str,
    expected: &str,
) -> Result<()> {
    ensure!(required(fields, key)? == expected, "legacy {key} conflicts");
    Ok(())
}
pub(super) fn optional_value(
    fields: &BTreeMap<String, String>,
    key: &str,
) -> Result<Option<String>> {
    let value = required(fields, key)?;
    Ok((!value.eq_ignore_ascii_case("not-applicable")).then(|| value.to_owned()))
}
pub(super) fn clean_index(value: &str) -> Result<(bool, bool)> {
    match value.trim() {
        "clean" | "dirty" | "index" | "dirty,index" | "index,dirty" => {
            Ok((value.contains("dirty"), value.contains("index")))
        }
        _ => bail!("terminal clean/index state is not canonical"),
    }
}
fn text(value: &str, field: &str, limit: usize) -> Result<()> {
    ensure!(
        !value.is_empty() && value.len() <= limit && !value.chars().any(char::is_control),
        "{field} must be bounded, non-empty, and free of control characters"
    );
    Ok(())
}
pub(super) fn digest_value<T: Serialize>(value: &T, prefix: &str) -> String {
    let bytes = serde_json::to_vec(value).unwrap_or_default();
    format!("{prefix}:{:x}", Sha256::digest(bytes))
}
