use std::path::Path;

use anyhow::{Context as _, Result, bail};
use serde::Deserialize;
use serde_json::{Map, Value};

use crate::paths::display_relative;
use crate::validation::load_json;

const CAPABILITY_PATH: &str = "hooks/postcompact-capability.json";
const SUPPORTED_BUILD: &str = "0.144.4";
const UPSTREAM_TAG: &str = "rust-v0.144.4";
const UPSTREAM_COMMIT: &str = "8c68d4c87dc54d38861f5114e920c3de2efa5876";
const UPSTREAM_TRACKER: &str = "https://github.com/eunsoogi/codexy/issues/455";
const PRE_EVIDENCE: &[&str] = &[
    "codex-rs/hooks/schema/generated/pre-compact.command.output.schema.json",
    "codex-rs/hooks/src/schema.rs:166-170",
    "codex-rs/hooks/src/events/compact.rs:230-280",
    "codex-rs/core/src/compact.rs:169-192",
    "codex-rs/core/src/hook_runtime.rs:368-392",
];
const POST_EVIDENCE: &[&str] = &[
    "codex-rs/hooks/schema/generated/post-compact.command.output.schema.json",
    "codex-rs/hooks/src/schema.rs:172-179",
    "codex-rs/hooks/src/engine/output_parser.rs:241-257",
    "codex-rs/hooks/src/events/compact.rs:315-416",
    "codex-rs/core/src/compact.rs:196-204",
    "codex-rs/core/src/hook_runtime.rs:405-429",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ContextEvent {
    None,
    PreCompact,
    PostCompact,
}

impl ContextEvent {
    pub(super) const fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::PreCompact => "PreCompact",
            Self::PostCompact => "PostCompact",
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CapabilityRecord {
    supported_codex_build: String,
    upstream_tag: String,
    upstream_commit: String,
    selection: SelectionRecord,
    pre_compact: CompactionRecord,
    post_compact: CompactionRecord,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SelectionRecord {
    semantic_default: String,
    selected_context_event: String,
    reason: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CompactionRecord {
    model_visible_developer_context: bool,
    context_survives_compaction_exactly_once: bool,
    manual_trigger_supported: bool,
    automatic_trigger_supported: bool,
    live_model_context_delivery_proven: bool,
    system_message_counts_as_model_context: bool,
    status: String,
    blocker: String,
    upstream_tracker: String,
    evidence: Vec<String>,
}

pub(super) fn check(plugin_root: &Path) -> Result<ContextEvent> {
    let path = plugin_root.join(CAPABILITY_PATH);
    let record: CapabilityRecord =
        serde_json::from_value(load_json(&path)?).with_context(|| {
            format!(
                "{} must match the compaction capability schema",
                display_relative(&path)
            )
        })?;
    check_identity(&path, &record)?;
    let pre = check_event(&path, "PreCompact", &record.pre_compact, PRE_EVIDENCE)?;
    let post = check_event(&path, "PostCompact", &record.post_compact, POST_EVIDENCE)?;
    check_selection(&path, &record.selection, pre, post)
}

pub(super) fn check_topology(
    hooks_path: &Path,
    events: &Map<String, Value>,
    selected: ContextEvent,
) -> Result<()> {
    for event in [ContextEvent::PreCompact, ContextEvent::PostCompact] {
        let configured = events.contains_key(event.as_str());
        if configured != (selected == event) {
            bail!(
                "{} must configure only the selected {} model-context event",
                display_relative(hooks_path),
                selected.as_str()
            );
        }
    }
    Ok(())
}

fn check_identity(path: &Path, record: &CapabilityRecord) -> Result<()> {
    if record.supported_codex_build != SUPPORTED_BUILD
        || record.upstream_tag != UPSTREAM_TAG
        || record.upstream_commit != UPSTREAM_COMMIT
    {
        bail!(
            "{} must preserve the exact supported Codex build, upstream tag, and commit",
            display_relative(path)
        );
    }
    Ok(())
}

fn check_event(
    path: &Path,
    event: &str,
    record: &CompactionRecord,
    evidence: &[&str],
) -> Result<bool> {
    if record.system_message_counts_as_model_context {
        bail!(
            "{} {event} systemMessage MUST NOT count as model-visible developer context",
            display_relative(path)
        );
    }
    for item in evidence {
        if !record.evidence.iter().any(|actual| actual == item) {
            bail!(
                "{} missing exact {event} evidence: {item}",
                display_relative(path)
            );
        }
    }
    if record.model_visible_developer_context {
        if !record.manual_trigger_supported
            || !record.automatic_trigger_supported
            || !record.live_model_context_delivery_proven
            || !record.context_survives_compaction_exactly_once
            || record.status != "proven"
        {
            bail!(
                "{} {event} support requires exact-once manual and auto model-context proof",
                display_relative(path)
            );
        }
        bail!(
            "{} pinned Codex 0.144.4 evidence proves {event} model context is unsupported",
            display_relative(path)
        );
    }
    if record.context_survives_compaction_exactly_once
        || !record.manual_trigger_supported
        || !record.automatic_trigger_supported
        || record.live_model_context_delivery_proven
        || record.status != "blocked-upstream"
        || record.blocker.trim().is_empty()
        || record.upstream_tracker != UPSTREAM_TRACKER
    {
        bail!(
            "{} blocked {event} support must distinguish supported manual/automatic triggers from absent live model-context delivery and preserve tracker #455",
            display_relative(path)
        );
    }
    Ok(false)
}

fn check_selection(
    path: &Path,
    selection: &SelectionRecord,
    pre: bool,
    post: bool,
) -> Result<ContextEvent> {
    if selection.semantic_default != "PostCompact" || selection.reason.trim().is_empty() {
        bail!(
            "{} selection must preserve PostCompact as the semantic default with a reason",
            display_relative(path)
        );
    }
    let expected = if post {
        ContextEvent::PostCompact
    } else if pre {
        ContextEvent::PreCompact
    } else {
        ContextEvent::None
    };
    if selection.selected_context_event != expected.as_str() {
        bail!(
            "{} selectedContextEvent {} must be {} from the exact capability proof",
            display_relative(path),
            selection.selected_context_event,
            expected.as_str()
        );
    }
    Ok(expected)
}
