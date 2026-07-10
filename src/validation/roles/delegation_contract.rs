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
    let permits_recursion = text.split(['.', '!', '?', '\n']).any(|clause| {
        let mut clause = clause.to_ascii_lowercase();
        if allow_canonical_child_delegation {
            clause = clause.replace(CANONICAL_CHILD_DELEGATION_PREFIX, "");
            clause = clause.replace(CANONICAL_ROOT_DELEGATION, "");
        }
        let action = ["spawn", "delegate", "create"]
            .into_iter()
            .any(|action| clause.contains(action));
        let target = ["agent", "helper", "reviewer", "task", "thread"]
            .into_iter()
            .any(|target| clause.contains(target));
        let permission = has_unnegated_permission(&clause);
        permission && action && target
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
        "may" | "can" => words.get(index + 1).is_none_or(|next| *next != "not"),
        "allowed" => words.get(index + 1) == Some(&"actions") && !words.contains(&"not"),
        "permitted" => {
            words
                .get(index + 1)
                .is_some_and(|next| matches!(*next, "to" | "actions"))
                && !words.contains(&"not")
        }
        _ => false,
    })
}
