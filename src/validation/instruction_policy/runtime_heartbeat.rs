use std::path::Path;

mod markdown;
mod weakening;

use crate::paths::display_relative;
use markdown::{last_modal_is_soft, normalized_policy_text};
use weakening::has_weakening_suffix;

const NORMALIZED_DISCOVERY_CLAUSE: &str = "search the callable tool surface for automation_update";
const LEGACY_CHILD_STATE_ELIGIBILITY: &str = "when github ci, review-thread state, child state, or another external gate will outlive the current turn, the owning parent orchestrator or child must search the callable tool surface for automation_update before declaring persistent monitoring unavailable";
const LEGACY_HEARTBEAT_REGISTRATION: &str = "the owner must register a heartbeat instead of repeated model continuations or ending without a wakeup path";
const RESTRICTED_HEARTBEAT_CONTEXT: &str =
    "for such genuinely scheduled monitoring or unavailable-wait fallback";

const ORCHESTRATION_CLAUSES: &[&str] = &[
    "MUST use event-driven `wait_threads` with each target's latest cursor as the default for ordinary child completion or attention waits",
    "MUST reserve heartbeat scheduling for genuinely scheduled monitoring or when `wait_threads` is unavailable",
    "After a host transition or `No handler registered` failure, the owner MUST treat the mismatch as host-transition exposure evidence, perform one fresh thread-tool discovery and one host-aware `wait_threads` retry before any fallback, MUST NOT use unbounded `read_thread`, and any bounded metadata fallback MUST consume the current parent-stage budget and record only returned size/token metadata",
    "When genuinely scheduled monitoring or an unavailable `wait_threads` route will outlive the current turn, the owning parent orchestrator or child MUST search the callable tool surface for `automation_update` before declaring persistent monitoring unavailable",
    "For such genuinely scheduled monitoring or unavailable-wait fallback, the owner MUST register a heartbeat instead of repeated model continuations or ending without a wakeup path",
    "search the callable tool surface for `automation_update`",
    "MUST use a thread-targeted `kind=heartbeat`",
    "creation MUST use `destination=\"thread\"`",
    "automation id, target thread, bounded schedule, stable observed-state identity, eligible material events, and terminal delete/disable action",
    "prompt MUST suppress unchanged observations and MUST wake the owner only for a material gate change or an explicit user/parent message",
    "MUST end its active goal and plan before waiting",
    "MUST NOT retain or recreate an execution goal solely to preserve a successfully registered heartbeat",
    "qualifying event MUST start a fresh short-lived execution goal and plan",
    "MUST consume the event in the same turn",
    "MUST delete or disable the heartbeat when no further observation is required",
    "MUST record the exact discovery/exposure evidence and use a bounded fallback",
    "without fabricating a monitor identity",
    "MUST mark automation id, schedule, and lifecycle as not-created",
    "MUST NOT fold a live packaged Sentinel into heartbeat observation",
    "read-only, event-driven, and subject to its no-poll/no-message boundary",
];

const TOKEN_CLAUSES: &[&str] = &[
    "MUST use event-driven `wait_threads` with each target's latest cursor as the default for ordinary child completion or attention waits",
    "MUST reserve heartbeat scheduling for genuinely scheduled monitoring or when `wait_threads` is unavailable",
    "After a host transition or `No handler registered` failure, the owner MUST treat the mismatch as host-transition exposure evidence, perform one fresh thread-tool discovery and one host-aware `wait_threads` retry before any fallback, MUST NOT use unbounded `read_thread`, and any bounded metadata fallback MUST consume the current parent-stage budget and record only returned size/token metadata",
    "polling/monitoring MUST be reserved for an observation bound to one complete runtime-issued monitor identity",
    "heartbeat route MUST bind the observation to its heartbeat automation id, target thread, bounded schedule, and last observed state fingerprint or event identity",
    "heartbeat route MUST NOT require a persistent exec/session identifier or same-process resume",
    "separate process-backed monitor MUST bind the observation to a persistent runtime monitor or wait session id, a scheduled next-observation time or deadline, the last observed state fingerprint or event identity, and same-process resume",
    "without either complete runtime-issued identity are continuation turns, not polling",
    "bounded schedule, state fingerprint, material-event set, and delete/disable state",
    "MUST suppress unchanged observations",
    "material gate change or an explicit user/parent message",
    "active goal and plan MUST end before runtime-owned waiting",
    "qualifying event MUST start a fresh short-lived execution goal and plan",
];

const EXTERNAL_GATE_CLAUSES: &[&str] = &[
    "MUST use event-driven `wait_threads` with each target's latest cursor as the default for ordinary child completion or attention waits",
    "MUST reserve heartbeat scheduling for genuinely scheduled monitoring or when `wait_threads` is unavailable",
    "After a host transition or `No handler registered` failure, the owner MUST treat the mismatch as host-transition exposure evidence, perform one fresh thread-tool discovery and one host-aware `wait_threads` retry before any fallback, MUST NOT use unbounded `read_thread`, and any bounded metadata fallback MUST consume the current parent-stage budget and record only returned size/token metadata",
    "MUST follow `references/runtime-heartbeats.md`",
    "parent or child MUST NOT retain an active goal or plan during an external-gate wait",
    "child external-gate wait MUST end its active goal and plan before waiting",
    "qualifying event starts a fresh short-lived execution goal",
    "heartbeat automation route MUST NOT require a persistent exec/session id or same-process resume",
];

const TEMPLATE_CLAUSES: &[&str] = &[
    "callable discovery/exposure evidence:",
    "heartbeat automation id:",
    "target thread:",
    "bounded schedule:",
    "state fingerprint:",
    "eligible material events:",
    "unchanged observations suppressed:",
    "terminal delete/disable action:",
];

const TRANSITION_CLAUSES: &[&str] = &[
    "heartbeat automation id, target thread, bounded schedule, and last observed state fingerprint or event identity",
    "MUST NOT require a persistent exec/session identifier or same-process resume",
    "persistent exec/session identifier, a scheduled next-observation deadline, the last observed state fingerprint or event identity, and same-process resume",
];

const CONDITIONAL_MARKERS: &[&str] = &[
    "unless ",
    "except ",
    "only if ",
    "when possible",
    "if available",
    "as needed",
];

pub(super) fn check(path: &Path, text: &str, errors: &mut Vec<String>) {
    let (requirement, clauses) = if path.ends_with("skills/codex-orchestration/SKILL.md") {
        (
            "orchestration skill must preserve the runtime heartbeat external-gate policy",
            EXTERNAL_GATE_CLAUSES,
        )
    } else if path.ends_with("skills/codex-orchestration/references/runtime-heartbeats.md") {
        (
            "runtime heartbeat contract must preserve its lifecycle policy",
            ORCHESTRATION_CLAUSES,
        )
    } else if path.ends_with("skills/token-efficient-orchestration/SKILL.md") {
        (
            "token-efficient skill must preserve the runtime heartbeat contract",
            TOKEN_CLAUSES,
        )
    } else if path.ends_with("skills/token-efficient-orchestration/templates/delta-poll.md") {
        (
            "runtime heartbeat delta template must preserve lifecycle slots",
            TEMPLATE_CLAUSES,
        )
    } else if path.ends_with("skills/codex-orchestration/references/goal-transition-reporting.md") {
        (
            "goal transition contract must distinguish heartbeat and process monitor identities",
            TRANSITION_CLAUSES,
        )
    } else {
        return;
    };
    let normalized = normalized_policy_text(text);
    for clause in clauses {
        let clause = normalized_policy_text(clause);
        if !has_unweakened_clause(&normalized, &clause) {
            errors.push(format!(
                "{} {requirement}: missing `{clause}`",
                display_relative(path)
            ));
        }
    }
    if path.ends_with("skills/codex-orchestration/references/runtime-heartbeats.md")
        && normalized.contains("may fold a live packaged sentinel into heartbeat observation")
    {
        errors.push(format!(
            "{} runtime heartbeat contract must not permit Sentinel heartbeat observation",
            display_relative(path)
        ));
    }
    if path.ends_with("skills/codex-orchestration/references/runtime-heartbeats.md")
        && has_unweakened_clause(&normalized, LEGACY_CHILD_STATE_ELIGIBILITY)
    {
        errors.push(format!(
            "{} runtime heartbeat contract must not retain unconditional heartbeat eligibility for child state",
            display_relative(path)
        ));
    }
    if path.ends_with("skills/codex-orchestration/references/runtime-heartbeats.md")
        && has_unconditional_clause(
            &normalized,
            LEGACY_HEARTBEAT_REGISTRATION,
            RESTRICTED_HEARTBEAT_CONTEXT,
        )
    {
        errors.push(format!(
            "{} runtime heartbeat contract must not retain unconditional heartbeat registration",
            display_relative(path)
        ));
    }
}

fn has_unweakened_clause(text: &str, clause: &str) -> bool {
    text.match_indices(clause).any(|(index, _)| {
        let before = &text[..index];
        let after = &text[index + clause.len()..];
        is_unweakened_clause(before, after, clause)
    })
}
fn has_unconditional_clause(text: &str, clause: &str, restriction: &str) -> bool {
    text.match_indices(clause).any(|(index, _)| {
        let before = &text[..index];
        let after = &text[index + clause.len()..];
        is_unweakened_clause(before, after, clause)
            && !current_sentence_prefix(before).contains(restriction)
    })
}

fn is_unweakened_clause(before: &str, after: &str, clause: &str) -> bool {
    has_clause_boundaries(before, after)
        && before.rfind("<markdown-heading>") <= before.rfind("</markdown-heading>")
        && !current_block_prefix(before)
            .rsplit(['.', ';'])
            .next()
            .is_some_and(|prefix| {
                [
                    "historical example",
                    "false that",
                    "not required",
                    "no longer required",
                ]
                .iter()
                .any(|marker| prefix.contains(marker))
            })
        && !has_conditional_context(before)
        && !has_negated_prefix(before)
        && !(clause == NORMALIZED_DISCOVERY_CLAUSE
            && last_modal_is_soft(current_sentence_prefix(before)))
        && !has_weakening_suffix(after, CONDITIONAL_MARKERS)
}

fn has_clause_boundaries(before: &str, after: &str) -> bool {
    before
        .chars()
        .next_back()
        .is_none_or(|character| !is_clause_token_character(character))
        && after
            .chars()
            .next()
            .is_none_or(|character| !is_clause_token_character(character))
}

fn is_clause_token_character(character: char) -> bool {
    character.is_alphanumeric() || matches!(character, '_' | '-')
}

fn has_negated_prefix(before: &str) -> bool {
    current_sentence_prefix(before)
        .trim_end()
        .ends_with("must not")
}

fn has_conditional_context(before: &str) -> bool {
    CONDITIONAL_MARKERS.iter().any(|marker| {
        current_sentence_prefix(before).contains(marker.trim())
            || current_heading(before).is_some_and(|heading| heading.contains(marker.trim()))
    })
}

fn current_heading(before: &str) -> Option<&str> {
    let (_, heading) = before.rsplit_once("<markdown-heading>")?;
    heading
        .split_once("</markdown-heading>")
        .map(|(heading, _)| heading)
}

fn current_block_prefix(before: &str) -> &str {
    let section = before
        .rsplit_once("</markdown-heading>")
        .map_or(before, |(_, current_section)| current_section);
    section
        .rsplit_once("<markdown-boundary>")
        .map_or(section, |(_, current_block)| current_block)
}

fn current_sentence_prefix(before: &str) -> &str {
    current_block_prefix(before)
        .rsplit(['.', ';'])
        .next()
        .unwrap_or_default()
        .trim_start()
}
