use super::child_lane_ownership_phrases::{field_value, metadata_key, trimmed_value};

pub(super) fn has_subagent_as_thread_owner(evidence: &str) -> bool {
    evidence
        .lines()
        .map(str::trim)
        .any(line_claims_subagent_thread_owner)
}

fn line_claims_subagent_thread_owner(line: &str) -> bool {
    if line.is_empty() || line_is_helper_only(line) {
        return false;
    }
    if let Some((key, value)) = line.split_once(':') {
        let key = metadata_key(key);
        return owner_key_requires_thread_owner(key) && value_claims_subagent_owner(key, value);
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
    if value_is_non_child_owned_decision_with_subagent_rationale(value) {
        return false;
    }
    if value_is_parent_owned_routing_only(value) {
        return false;
    }
    if value_denies_subagent_owner(value) {
        return false;
    }
    if thread_owner_key(key) {
        return !has_true_codex_thread_owner(value);
    }
    !has_true_codex_thread_owner(value)
}

fn value_is_parent_owned_routing_only(value: &str) -> bool {
    let value = trimmed_value(value);
    value.contains("parent-owned")
        && (value.contains("routing")
            || value.contains("tool discovery")
            || value.contains("thread/worktree tool discovery"))
        && value_has_non_owner_subagent_rationale(value)
}

fn value_is_non_child_owned_decision_with_subagent_rationale(value: &str) -> bool {
    let value = trimmed_value(value);
    (value.contains("parent-owned") || value.contains("current-thread-owned"))
        && value_has_non_owner_subagent_rationale(value)
}

fn value_has_non_owner_subagent_rationale(value: &str) -> bool {
    value_denies_subagent_owner(value)
        || [
            "subagent not useful",
            "sub-agent not useful",
            "multi_agent not useful",
            "multi-agent not useful",
            "specialist helper not useful",
        ]
        .into_iter()
        .any(|marker| value.contains(marker))
}

fn owner_key_requires_thread_owner(key: &str) -> bool {
    [
        "owner",
        "owner decision",
        "child owner",
        "lane owner",
        "subthread/worktree owner",
        "thread/worktree owner",
        "subthread owner",
        "worktree owner",
    ]
    .into_iter()
    .any(|field| key == field || key.contains(field))
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
    ]
    .into_iter()
    .any(|marker| value.contains(marker))
}

fn has_subagent_owner_assignment(value: &str) -> bool {
    let value = trimmed_value(value);
    [
        "assigned to subagent",
        "assigned to sub-agent",
        "assigned to multi_agent",
        "assigned to multi-agent",
        "routed to subagent",
        "routed to sub-agent",
        "routed to multi_agent",
        "routed to multi-agent",
        "owned by subagent",
        "owned by multi_agent",
        "owned by multi-agent",
        "subagent owner",
        "multi_agent owner",
        "multi-agent owner",
        "spawn_agent owner",
    ]
    .into_iter()
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
    [
        "no codex worktree thread",
        "no codex child thread",
        "no codex thread",
        "no worktree thread",
        "no child thread",
        "codex worktree thread unavailable",
        "codex child thread unavailable",
        "codex thread unavailable",
        "codex thread tools unavailable",
        "worktree thread unavailable",
        "child thread unavailable",
        "not codex worktree thread",
        "not codex child thread",
        "not codex thread",
        "not worktree thread",
        "not child thread",
        "without codex worktree thread",
        "without codex child thread",
        "without codex thread",
        "without worktree thread",
        "without child thread",
    ]
    .into_iter()
    .any(|marker| value.contains(marker))
}

fn thread_owner_key(key: &str) -> bool {
    [
        "child owner",
        "lane owner",
        "subthread/worktree owner",
        "thread/worktree owner",
        "subthread owner",
        "worktree owner",
    ]
    .into_iter()
    .any(|field| key == field || key.contains(field))
}

fn value_denies_subagent_owner(value: &str) -> bool {
    let value = trimmed_value(value);
    [
        "not the owner",
        "not owner",
        "not implementation owner",
        "not a child owner",
        "not a subthread owner",
        "not a worktree owner",
        "no subagent substitute",
        "no sub-agent substitute",
        "no multi_agent substitute",
        "no multi-agent substitute",
        "not a subagent substitute",
        "not a sub-agent substitute",
        "not a multi_agent substitute",
        "not a multi-agent substitute",
        "subagent substitute not used",
        "sub-agent substitute not used",
        "multi_agent substitute not used",
        "multi-agent substitute not used",
        "subagent fallback not used",
        "sub-agent fallback not used",
        "multi_agent fallback not used",
        "multi-agent fallback not used",
        "helper only",
        "reviewer only",
    ]
    .into_iter()
    .any(|marker| value.contains(marker))
}
