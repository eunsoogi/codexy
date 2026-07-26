use std::path::Path;

mod clauses;
mod markdown;
mod weakening;

use crate::paths::display_relative;
use clauses::{
    CONDITIONAL_MARKERS, EXTERNAL_GATE, LEGACY_CHILD_STATE_ELIGIBILITY,
    LEGACY_HEARTBEAT_REGISTRATION, ORCHESTRATION, RESTRICTED_HEARTBEAT_CONTEXT, TEMPLATE, TOKEN,
    TRANSITION,
};
use markdown::{last_modal_is_soft, normalized_policy_text};
use weakening::has_weakening_suffix;

const NORMALIZED_DISCOVERY_CLAUSE: &str = "search the callable tool surface for automation_update";

pub(super) fn check(path: &Path, text: &str, errors: &mut Vec<String>) {
    let (requirement, clauses) = if path.ends_with("skills/codex-orchestration/SKILL.md") {
        (
            "orchestration skill must preserve the runtime heartbeat external-gate policy",
            EXTERNAL_GATE,
        )
    } else if path.ends_with("skills/codex-orchestration/references/runtime-heartbeats.md") {
        (
            "runtime heartbeat contract must preserve its lifecycle policy",
            ORCHESTRATION,
        )
    } else if path.ends_with("skills/token-efficient-orchestration/SKILL.md") {
        (
            "token-efficient skill must preserve the runtime heartbeat contract",
            TOKEN,
        )
    } else if path.ends_with("skills/token-efficient-orchestration/templates/delta-poll.md") {
        (
            "runtime heartbeat delta template must preserve lifecycle slots",
            TEMPLATE,
        )
    } else if path.ends_with("skills/codex-orchestration/references/goal-transition-reporting.md") {
        (
            "goal transition contract must distinguish heartbeat and process monitor identities",
            TRANSITION,
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
