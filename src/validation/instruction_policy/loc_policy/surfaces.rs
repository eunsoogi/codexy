use std::path::Path;

pub(super) const GOVERNED_SKILLS: &[&str] = &[
    "skills/git-workflow/SKILL.md",
    "skills/plugin-marketplace-prep/SKILL.md",
    "skills/proof-driven-completion/SKILL.md",
    "skills/refactoring/SKILL.md",
];
pub(super) const GOVERNED_AGENT_ROLES: &[&str] = &["agents/codexy-sculptor.toml"];
pub(super) const UNCONDITIONAL_CONTRACT: &str = "every governed file MUST stay at or below 250 LOC";
pub(super) const EXCEPTION_PROHIBITION: &str = "MUST NOT use or authorize LOC exceptions";

pub(super) fn is_governed_root_agents(path: &Path) -> bool {
    path.file_name().is_some_and(|name| name == "AGENTS.md")
}
