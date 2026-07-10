use std::{fs, path::Path};

use toml::Value;

use crate::paths::display_relative;

const NO_RECURSIVE_DELEGATION: &str = "MUST NOT spawn, delegate to, or create any additional agent, helper, reviewer, task, or thread.";
const ALLOWED_CHILD_DELEGATION: &str = "A child implementation thread MAY spawn bounded first-level specialist helpers or Sentinel reviewers.";
const CANONICAL_CHILD_DELEGATION_PREFIX: &str =
    "a child implementation thread may spawn bounded first-level specialist helpers";
const CANONICAL_ROOT_DELEGATION: &str = "the root orchestrator may create child threads";
const SPAWN_EXAMPLES: &[&str] = &[
    "spawn_agent(agent_type=\"codexy-sentinel\", message=\"Review the current diff, exact head, scope, verification output, and evidence. MUST NOT spawn, delegate to, or create any additional agent, helper, reviewer, task, or thread.\"",
    "spawn_agent(agent_type=\"codexy-pathfinder\", message=\"Produce an atomic plan and verification checklist. MUST NOT spawn, delegate to, or create any additional agent, helper, reviewer, task, or thread.\"",
    "spawn_agent(agent_type=\"codexy-cartographer\", message=\"Map the relevant files. MUST NOT spawn, delegate to, or create any additional agent, helper, reviewer, task, or thread.\"",
];
const ORCHESTRATION_REFERENCES: &[(&str, &str)] = &[
    (
        "skills/codex-orchestration/references/classification-and-control.md",
        "every helper or Sentinel MUST NOT spawn, delegate to, or create any additional agent, helper, reviewer, task, or thread.",
    ),
    (
        "skills/codex-orchestration/references/orchestration-loop.md",
        "Every helper or Sentinel assignment MUST include the nonrecursive delegation prohibition.",
    ),
];

pub(super) fn check(path: &Path, agent: &Value, errors: &mut Vec<String>) {
    let instructions = agent
        .get("developer_instructions")
        .and_then(Value::as_str)
        .unwrap_or("");
    if !instructions.contains(NO_RECURSIVE_DELEGATION) {
        errors.push(format!(
            "{} nonrecursive delegation contract is missing: {NO_RECURSIVE_DELEGATION}",
            display_relative(path)
        ));
    }
    reject_recursive_delegation_permission(path, instructions, false, errors);
}

pub(super) fn check_orchestration_contract(plugin_root: &Path, errors: &mut Vec<String>) {
    let path = plugin_root.join("skills/codex-orchestration/SKILL.md");
    let Ok(skill) = fs::read_to_string(&path) else {
        errors.push(format!(
            "{} nonrecursive delegation contract cannot be read",
            display_relative(&path)
        ));
        return;
    };
    for marker in [ALLOWED_CHILD_DELEGATION, NO_RECURSIVE_DELEGATION]
        .into_iter()
        .chain(SPAWN_EXAMPLES.iter().copied())
    {
        if !skill.contains(marker) {
            errors.push(format!(
                "{} nonrecursive delegation contract is missing: {marker}",
                display_relative(&path)
            ));
        }
    }
    reject_recursive_delegation_permission(&path, &skill, true, errors);
    for &(relative_path, marker) in ORCHESTRATION_REFERENCES {
        let path = plugin_root.join(relative_path);
        let Ok(reference) = fs::read_to_string(&path) else {
            errors.push(format!(
                "{} nonrecursive delegation contract cannot be read",
                display_relative(&path)
            ));
            continue;
        };
        if !reference.contains(marker) || !reference.contains(NO_RECURSIVE_DELEGATION) {
            errors.push(format!(
                "{} nonrecursive delegation contract is missing required boundary text",
                display_relative(&path)
            ));
        }
        reject_recursive_delegation_permission(&path, &reference, true, errors);
    }
}

fn reject_recursive_delegation_permission(
    path: &Path,
    text: &str,
    allow_canonical_child_delegation: bool,
    errors: &mut Vec<String>,
) {
    let normalized = normalize_instruction_text(text);
    let permits_recursion = normalized.split(['.', '!', '?']).any(|clause| {
        let mut clause = clause.to_ascii_lowercase();
        let mut inherited_child_permission = false;
        if allow_canonical_child_delegation {
            inherited_child_permission = clause.contains(CANONICAL_CHILD_DELEGATION_PREFIX);
            clause = clause.replace(CANONICAL_CHILD_DELEGATION_PREFIX, "");
            clause = clause.replace(CANONICAL_ROOT_DELEGATION, "");
        }
        let action = has_unnegated_delegation_action(&clause);
        (inherited_child_permission
            || has_unnegated_permission(&clause)
            || has_unnegated_mandatory_delegation_action(&clause, allow_canonical_child_delegation))
            && action
    });
    if permits_recursion {
        errors.push(format!(
            "{} nonrecursive delegation contract permits recursive delegation",
            display_relative(path)
        ));
    }
}

fn has_unnegated_permission(clause: &str) -> bool {
    let words = clause
        .split(|character: char| !character.is_ascii_alphabetic())
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>();
    words.iter().enumerate().any(|(index, word)| match *word {
        "may" | "can" => words
            .get(index + 1)
            .is_none_or(|next| !matches!(*next, "not" | "never")),
        "allowed" => {
            words.get(index.wrapping_sub(1)) != Some(&"not")
                && words
                    .get(index + 1)
                    .is_some_and(|next| matches!(*next, "actions" | "to"))
        }
        "permitted" => {
            words.get(index.wrapping_sub(1)) != Some(&"not")
                && words
                    .get(index + 1)
                    .is_some_and(|next| matches!(*next, "to" | "actions"))
        }
        _ => false,
    })
}

fn has_unnegated_delegation_action(clause: &str) -> bool {
    ["spawn", "delegate", "create"].into_iter().any(|action| {
        clause.match_indices(action).any(|(index, _)| {
            let prefix = &clause[..index];
            let action_prefix = prefix
                .rsplit_once(" but ")
                .map_or(prefix, |(_, contrast)| contrast);
            let negated = has_action_negation(action_prefix);
            let suffix = &clause[index..];
            let target = [
                "agent",
                "helper",
                "reviewer",
                "sentinel",
                "specialist",
                "task",
                "thread",
            ]
            .into_iter()
            .any(|target| suffix.contains(target));
            !negated && target
        })
    })
}

fn has_unnegated_mandatory_delegation_action(
    clause: &str,
    allow_root_child_thread_creation: bool,
) -> bool {
    ["spawn", "delegate", "create"].into_iter().any(|action| {
        clause.match_indices(action).any(|(index, _)| {
            let prefix = clause[..index]
                .rsplit_once(" but ")
                .map_or(&clause[..index], |(_, contrast)| contrast);
            let suffix = &clause[index..];
            let creates_child_thread = allow_root_child_thread_creation
                && action == "create"
                && clause.contains("orchestrator")
                && suffix.contains("child thread");
            has_unnegated_mandatory_permission(prefix)
                && !creates_child_thread
                && [
                    "agent",
                    "helper",
                    "reviewer",
                    "sentinel",
                    "specialist",
                    "task",
                    "thread",
                ]
                .into_iter()
                .any(|target| suffix.contains(target))
        })
    })
}

fn has_unnegated_mandatory_permission(prefix: &str) -> bool {
    let words = prefix
        .split(|character: char| !character.is_ascii_alphabetic())
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>();
    words
        .iter()
        .rposition(|word| *word == "must")
        .is_some_and(|index| words.get(index + 1) != Some(&"not"))
}

fn has_action_negation(prefix: &str) -> bool {
    let words = prefix
        .split(|character: char| !character.is_ascii_alphabetic())
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>();
    let Some(index) = words
        .iter()
        .rposition(|word| matches!(*word, "may" | "can" | "must" | "allowed" | "permitted"))
    else {
        return false;
    };
    match words[index] {
        "may" | "can" => words
            .get(index + 1)
            .is_some_and(|next| matches!(*next, "not" | "never")),
        "must" => words.get(index + 1) == Some(&"not"),
        "allowed" | "permitted" => words.get(index.wrapping_sub(1)) == Some(&"not"),
        _ => false,
    }
}

fn normalize_instruction_text(text: &str) -> String {
    text.lines()
        .map(str::trim)
        .map(|line| {
            line.strip_prefix("- ")
                .or_else(|| line.strip_prefix("* "))
                .unwrap_or(line)
        })
        .map(|line| {
            line.split_once(". ")
                .filter(|(prefix, _)| prefix.chars().all(|character| character.is_ascii_digit()))
                .map_or(line, |(_, remainder)| remainder)
        })
        .collect::<Vec<_>>()
        .join(" ")
}
