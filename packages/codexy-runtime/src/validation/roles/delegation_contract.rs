use std::{collections::BTreeSet, fs, path::Path};

use toml::Value;

use crate::paths::display_relative;

use super::delegation_contract_parser::{
    has_unnegated_delegation_action, has_unnegated_mandatory_delegation_action,
    has_unnegated_permission, normalize_instruction_text,
};

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
const ORCHESTRATION_REFERENCE_ROOT: &str = "skills/orchestration";
const REGISTERED_REFERENCES: &[&str] = &[
    "references/task-classification.md",
    "references/classification-and-control.md",
    "references/goal-transition-reporting.md",
    "references/thread-and-worktree-routing.md",
    "references/orchestration-loop.md",
    "references/runtime-heartbeats.md",
    "references/parent-stop-preflight.md",
    "references/execution-budget.md",
    "references/token-efficient.md",
    "references/plain-language-user-replies.md",
    "references/natural-korean-responses.md",
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
    let path = plugin_root.join("skills/orchestration/SKILL.md");
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
    let references = match registered_orchestration_references(&skill) {
        Ok(references) => references,
        Err(error) => {
            errors.push(format!(
                "{} nonrecursive delegation references are invalid: {error}",
                display_relative(&path)
            ));
            return;
        }
    };
    for relative_path in references {
        let path = plugin_root.join(&relative_path);
        let Ok(reference) = fs::read_to_string(&path) else {
            errors.push(format!(
                "{} nonrecursive delegation contract cannot be read",
                display_relative(&path)
            ));
            continue;
        };
        reject_recursive_delegation_permission(&path, &reference, true, errors);
    }
}

fn registered_orchestration_references(skill: &str) -> Result<Vec<String>, String> {
    let section = skill
        .split_once("## Read Next")
        .and_then(|(_, remainder)| remainder.split_once("## Classification Gate"))
        .map(|(section, _)| section)
        .ok_or_else(|| "Read Next section is missing".to_owned())?;
    let parts = section.split('`').collect::<Vec<_>>();
    if parts.len() % 2 == 0 {
        return Err("Read Next contains an unmatched backtick".into());
    }
    let mut references = BTreeSet::new();
    for reference in parts.iter().skip(1).step_by(2) {
        if !REGISTERED_REFERENCES.contains(reference) {
            return Err(format!("unknown or retired reference: {reference}"));
        }
        if !references.insert(*reference) {
            return Err(format!("duplicate reference: {reference}"));
        }
    }
    let expected = REGISTERED_REFERENCES
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    if references != expected {
        return Err("reference inventory is incomplete".into());
    }
    Ok(references
        .into_iter()
        .map(|reference| format!("{ORCHESTRATION_REFERENCE_ROOT}/{reference}"))
        .collect())
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
        let action = has_unnegated_delegation_action(
            &clause,
            allow_canonical_child_delegation,
            inherited_child_permission,
        );
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
