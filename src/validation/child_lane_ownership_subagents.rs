use super::child_lane_ownership_phrases::{
    field_value, has_absent_field_value, metadata_key, trimmed_value,
};
use super::child_lane_ownership_subagent_format::{
    has_helper_only_purpose, has_unavailable_helper_rationale, key_allows_list_metadata_boundary,
    line_is_list_item, strip_list_marker,
};

const CODEXY_SPECIALIST_AGENTS: &str = "codexy-architect codexy-auditor codexy-cartographer codexy-forge codexy-pathfinder codexy-scribe codexy-sculptor codexy-sentinel codexy-shipwright codexy-tracer codexy-warden codexy-weaver";
const SUBAGENT_OWNER_ACTION_MARKERS: &str = "assigned to subagent|assigned to sub-agent|assigned to multi_agent|assigned to multi-agent|routed to subagent|routed to sub-agent|routed to multi_agent|routed to multi-agent|owned by subagent|owned by multi_agent|owned by multi-agent";
const SUBAGENT_OWNER_ACTION_DENIAL_MARKERS: &str = "not assigned to subagent|not assigned to sub-agent|not assigned to multi_agent|not assigned to multi-agent|not assigned to spawn_agent|not routed to subagent|not routed to sub-agent|not routed to multi_agent|not routed to multi-agent|not routed to spawn_agent|not owned by subagent|not owned by multi_agent|not owned by multi-agent|not owned by spawn_agent";
const SUBAGENT_OWNER_LABEL_MARKERS: &str =
    "subagent owner|multi_agent owner|multi-agent owner|spawn_agent owner";
const SUBAGENT_OWNER_DENIAL_MARKERS: &str = "not the owner|not owner|not implementation owner|not a child owner|not a subthread owner|not a worktree owner|no subagent owner|no sub-agent owner|no multi_agent owner|no multi-agent owner|no spawn_agent owner|subagent owner not used|sub-agent owner not used|multi_agent owner not used|multi-agent owner not used|spawn_agent owner not used|no subagent substitute|no sub-agent substitute|no multi_agent substitute|no multi-agent substitute|no spawn_agent substitute|not a subagent substitute|not a sub-agent substitute|not a multi_agent substitute|not a multi-agent substitute|not a spawn_agent substitute|subagent substitute not used|sub-agent substitute not used|multi_agent substitute not used|multi-agent substitute not used|spawn_agent substitute not used|subagent fallback not used|sub-agent fallback not used|multi_agent fallback not used|multi-agent fallback not used|spawn_agent fallback not used";
const NEGATED_CODEX_THREAD_OWNER_MARKERS: &str = "no codex worktree thread|no codex child thread|no codex thread|no worktree thread|no child thread|codex worktree thread unavailable|codex child thread unavailable|codex thread unavailable|codex thread tools unavailable|worktree thread unavailable|child thread unavailable|thread not available|thread was not available|not codex worktree thread|not codex child thread|not codex thread|not worktree thread|not child thread|not a codex worktree thread|not a codex child thread|not a codex thread|not a worktree thread|not a child thread|without codex worktree thread|without codex child thread|without codex thread|without worktree thread|without child thread|instead of codex worktree thread|instead of codex child thread|instead of codex thread|instead of worktree thread|instead of child thread|rather than codex worktree thread|rather than codex child thread|rather than codex thread|rather than worktree thread|rather than child thread";

pub(super) fn has_subagent_as_thread_owner(evidence: &str) -> bool {
    let mut owner_context: Option<(&str, String)> = None;
    for line in evidence.lines().map(str::trim) {
        if line.is_empty() {
            owner_context = None;
            continue;
        }
        let owner_scoped_subagent_metadata =
            owner_context.is_some() && line_is_owner_scoped_subagent_list_metadata(line);
        if line_is_helper_only(line) && !owner_scoped_subagent_metadata {
            owner_context = None;
            continue;
        }
        if owner_context
            .as_ref()
            .is_some_and(|(_, value)| !trimmed_value(value).is_empty())
            && line_starts_metadata_boundary(line)
            && !owner_scoped_subagent_metadata
        {
            owner_context = None;
        }
        if owner_context.is_none() {
            if let Some((key, value)) = line.split_once(':') {
                let key = metadata_key(key);
                if owner_key_requires_thread_owner(key) {
                    owner_context = Some((key, value.to_owned()));
                    if value_claims_subagent_owner(key, &format!("{key} {value}")) {
                        return true;
                    }
                } else {
                    owner_context = None;
                }
                continue;
            }
        }
        if let Some((key, value)) = owner_context.as_mut() {
            value.push(' ');
            value.push_str(line);
            if value_claims_subagent_owner(key, value) {
                return true;
            }
        }
    }
    false
}

fn value_claims_subagent_owner(key: &str, value: &str) -> bool {
    if !has_subagent_surface(value) {
        return false;
    }
    if has_subagent_owner_assignment(value) {
        return true;
    }
    let has_bare_subagent_owner_claim = trimmed_value(value)
        .split([';', ','])
        .flat_map(|clause| clause.split(" and "))
        .any(|clause| {
            let clause = trimmed_value(clause);
            has_subagent_surface(clause)
                && !has_true_codex_thread_owner(clause)
                && !value_has_non_owner_subagent_rationale(clause)
        });
    if has_bare_subagent_owner_claim {
        return true;
    }
    if value_is_non_child_owned_decision_with_subagent_rationale(value) {
        return false;
    }
    if value_denies_subagent_owner(value) || value_denies_subagent_owner_assignment(value) {
        return false;
    }
    if thread_owner_key(key) {
        return !has_true_codex_thread_owner(value)
            || !value_has_non_owner_subagent_rationale(value);
    }
    !has_true_codex_thread_owner(value) || !value_has_non_owner_subagent_rationale(value)
}

fn value_is_non_child_owned_decision_with_subagent_rationale(value: &str) -> bool {
    let value = trimmed_value(value);
    (value.contains("parent-owned") || value.contains("current-thread-owned"))
        && value_has_non_owner_subagent_rationale(value)
}

fn value_has_non_owner_subagent_rationale(value: &str) -> bool {
    value_denies_subagent_owner(value)
        || value_denies_subagent_owner_assignment(value)
        || "subagent not useful|sub-agent not useful|multi_agent not useful|multi-agent not useful|specialist helper not useful"
            .split('|')
            .any(|marker| value.contains(marker))
        || value.contains("reviewer gate required")
        || has_unavailable_helper_rationale(value)
        || has_helper_only_purpose(value)
}

fn owner_key_requires_thread_owner(key: &str) -> bool {
    if key.contains("non-owner") || key.contains("non owner") {
        return false;
    }
    key.contains("owner")
}

fn line_is_helper_only(line: &str) -> bool {
    if line
        .split_once(':')
        .is_some_and(|(key, _)| owner_key_requires_thread_owner(metadata_key(key)))
    {
        return false;
    }
    field_value(line, "specialist helper")
        .or_else(|| field_value(line, "subagent helper"))
        .or_else(|| field_value(line, "reviewer gate"))
        .is_some()
        || ["helper", "reviewer", "sentinel"]
            .into_iter()
            .any(|marker| {
                line.split_once(':')
                    .is_some_and(|(key, _)| metadata_key(key).contains(marker))
            })
}

fn line_starts_metadata_boundary(line: &str) -> bool {
    line.split_once(':').is_some_and(|(key, _)| {
        let key = metadata_key(strip_list_marker(key));
        !key.is_empty() && (!line_is_list_item(line) || key_allows_list_metadata_boundary(key))
    })
}

fn line_is_owner_scoped_subagent_list_metadata(line: &str) -> bool {
    line_is_list_item(line)
        && line.split_once(':').is_some_and(|(key, value)| {
            let key = metadata_key(strip_list_marker(key));
            has_subagent_surface(key) || (owner_scoped_role_key(key) && has_subagent_surface(value))
        })
}

fn owner_scoped_role_key(key: &str) -> bool {
    matches!(key, "agent" | "worker" | "reviewer" | "helper" | "explorer")
}

fn has_subagent_surface(value: &str) -> bool {
    let value = trimmed_value(value);
    [
        "subagent",
        "sub-agent",
        "multi_agent",
        "multi-agent",
        "spawn_agent",
        "spawned agent",
        "specialist agent",
        "specialist helper",
        "worker agent",
        "explorer agent",
        "reviewer agent",
        "reviewer gate",
    ]
    .into_iter()
    .any(|marker| value.contains(marker))
        || value_has_codexy_specialist_agent(value)
}

fn value_has_codexy_specialist_agent(value: &str) -> bool {
    let value = trimmed_value(value);
    CODEXY_SPECIALIST_AGENTS
        .split_whitespace()
        .any(|marker| value.contains(marker))
}

fn has_subagent_owner_assignment(value: &str) -> bool {
    let value = trimmed_value(value);
    SUBAGENT_OWNER_ACTION_MARKERS
        .split('|')
        .any(|marker| value_has_unnegated_marker(value, marker))
        || (!value_denies_subagent_owner(value)
            && SUBAGENT_OWNER_LABEL_MARKERS
                .split('|')
                .any(|marker| value.contains(marker)))
}

fn value_has_unnegated_marker(value: &str, marker: &str) -> bool {
    value.match_indices(marker).any(|(index, _)| {
        !value[..index]
            .trim_end_matches(|character: char| {
                character.is_ascii_whitespace() || matches!(character, ',' | ';')
            })
            .ends_with("not")
    })
}

fn value_denies_subagent_owner_assignment(value: &str) -> bool {
    SUBAGENT_OWNER_ACTION_DENIAL_MARKERS
        .split('|')
        .any(|marker| value.contains(marker))
}

fn has_true_codex_thread_owner(value: &str) -> bool {
    let value = trimmed_value(value);
    if negates_codex_thread_owner(value) {
        return false;
    }
    [
        "codex worktree thread",
        "codex child thread",
        "codex thread",
        "worktree thread",
        "child thread",
    ]
    .into_iter()
    .any(|marker| value.contains(marker))
}

fn negates_codex_thread_owner(value: &str) -> bool {
    NEGATED_CODEX_THREAD_OWNER_MARKERS
        .split('|')
        .any(|marker| value.contains(marker))
}

fn thread_owner_key(key: &str) -> bool {
    "child owner|lane owner|subthread/worktree owner|thread/worktree owner|subthread owner|worktree owner"
        .split('|')
        .any(|field| key == field || key.contains(field))
}

fn value_denies_subagent_owner(value: &str) -> bool {
    let value = trimmed_value(value);
    SUBAGENT_OWNER_DENIAL_MARKERS
        .split('|')
        .any(|marker| value.contains(marker))
        || SUBAGENT_OWNER_LABEL_MARKERS.split('|').any(|marker| {
            value
                .strip_prefix(marker)
                .is_some_and(|suffix| has_absent_field_value(suffix, marker))
        })
}
