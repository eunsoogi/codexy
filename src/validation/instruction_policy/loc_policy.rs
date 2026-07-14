use std::path::Path;

use crate::paths::display_relative;

const GOVERNED_SKILLS: &[&str] = &[
    "skills/git-workflow/SKILL.md",
    "skills/plugin-marketplace-prep/SKILL.md",
    "skills/proof-driven-completion/SKILL.md",
    "skills/refactoring/SKILL.md",
];
const UNCONDITIONAL_CONTRACT: &str = "every governed file MUST stay at or below 250 LOC";
const EXCEPTION_PROHIBITION: &str = "MUST NOT use or authorize LOC exceptions";

pub(super) fn check(path: &Path, text: &str, errors: &mut Vec<String>) {
    if !GOVERNED_SKILLS.iter().any(|skill| path.ends_with(skill)) {
        return;
    }
    if !contains_clause(text, UNCONDITIONAL_CONTRACT)
        || !contains_clause(text, EXCEPTION_PROHIBITION)
    {
        errors.push(format!(
            "{} LOC exception policy contract failed: missing unconditional governed 250 LOC clause",
            display_relative(path)
        ));
    }
    if text.lines().any(|line| {
        let line = line.to_ascii_lowercase();
        line.contains("loc exception")
            && ["unless", "tracked", "may", "exempt", "allow"]
                .iter()
                .any(|marker| line.contains(marker))
    }) {
        errors.push(format!(
            "{} LOC exception policy contract failed: must not allow LOC exceptions",
            display_relative(path)
        ));
    }
}

fn contains_clause(text: &str, clause: &str) -> bool {
    text.to_ascii_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .contains(
            &clause
                .to_ascii_lowercase()
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" "),
        )
}
