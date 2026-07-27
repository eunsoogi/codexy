use std::path::{Path, PathBuf};

use crate::support::{self, PluginFixture, TestResult};

pub(super) const GOVERNED_SKILLS: &[&str] = &[
    "skills/git-workflow/SKILL.md",
    "skills/plugin-marketplace-prep/SKILL.md",
    "skills/proof-driven-completion/SKILL.md",
    "skills/refactoring/SKILL.md",
];
pub(super) const REFERENCE_PATH: &str = "skills/wiki/references/loc-policy.md";

pub(super) fn plugin_fixture() -> TestResult<PluginFixture> {
    let mutable_files = GOVERNED_SKILLS.iter().map(Path::new).collect::<Vec<_>>();
    Ok(support::plugin_fixture_with_mutable_files(&mutable_files)?)
}

pub(super) fn reset_text(
    fixture: &PluginFixture,
    relative: &str,
) -> std::io::Result<(PathBuf, String)> {
    fixture.reset_file(Path::new(relative))?;
    let path = fixture.root().join(relative);
    Ok((path.clone(), std::fs::read_to_string(path)?))
}
