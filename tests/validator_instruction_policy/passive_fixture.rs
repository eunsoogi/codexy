use std::path::{Path, PathBuf};

use crate::support;

pub(super) fn copy_plugin_fixture(
) -> Result<(tempfile::TempDir, PathBuf), Box<dyn std::error::Error>> {
    Ok(support::copy_plugin_fixture_with_mutable_files(&[
        Path::new("skills/qa/SKILL.md"),
        Path::new("agents/codexy-shipwright.toml"),
        Path::new("skills/agents-md-authoring/SKILL.md"),
        Path::new("skills/codex-orchestration/SKILL.md"),
        Path::new("skills/proof-driven-completion/SKILL.md"),
        Path::new("skills/debugging/SKILL.md"),
        Path::new("skills/codex-orchestration/references/orchestration-loop.md"),
    ])?)
}
