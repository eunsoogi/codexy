use std::path::Path;

use crate::support::{self, PluginFixture, TestResult};

const MUTABLE_FILES: &[&str] = &[
    "agents/codexy-sentinel.toml",
    "skills/codex-orchestration/SKILL.md",
    "skills/proof-driven-completion/SKILL.md",
    "skills/token-efficient-orchestration/SKILL.md",
];
pub(super) const LIVE_OBSERVATION_SKILLS: &[&str] = &[
    "codex-orchestration",
    "proof-driven-completion",
    "token-efficient-orchestration",
];
pub(super) const LIVE_OBSERVATION_CLAUSE: &str =
    "Live Sentinel observation MUST be read-only and event-driven.";

pub(super) fn fixture() -> TestResult<PluginFixture> {
    let mutable_files = MUTABLE_FILES.iter().map(Path::new).collect::<Vec<_>>();
    Ok(support::plugin_fixture_with_mutable_files(&mutable_files)?)
}
