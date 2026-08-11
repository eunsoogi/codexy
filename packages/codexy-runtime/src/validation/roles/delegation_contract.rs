use std::path::Path;

use toml::Value;

use crate::paths::display_relative;

use super::delegation_contract_parser::{
    has_unnegated_delegation_action, has_unnegated_mandatory_delegation_action,
    has_unnegated_permission, normalize_instruction_text,
};

const NO_RECURSIVE_DELEGATION: &str = "MUST NOT spawn, delegate to, or create any additional agent, helper, reviewer, task, or thread.";
const CANONICAL_CHILD_DELEGATION_PREFIX: &str =
    "a child implementation thread may spawn bounded first-level specialist helpers";
const CANONICAL_ROOT_DELEGATION: &str = "the root orchestrator may create child threads";
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
