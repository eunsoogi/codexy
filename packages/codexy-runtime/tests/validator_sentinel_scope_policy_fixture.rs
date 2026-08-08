use std::path::Path;

use crate::support::{self, PluginFixture, TestResult};

const MUTABLE_FILES: &[&str] = &[
    "agents/codexy-sentinel.toml",
    "skills/codex-orchestration/SKILL.md",
    "skills/codex-orchestration/references/classification-and-control.md",
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
pub(super) const NONTERMINAL_OBSERVATION_CLAUSE: &str = "A bounded wait with no event is a non-terminal `PENDING` observation, and an independently observed live reviewer is `RUNNING`; neither observation is a reviewer verdict or fallback-eligible. The owning lane MUST retain the same reviewer and wait for its natural terminal result.";

pub(super) fn fixture() -> TestResult<PluginFixture> {
    let mutable_files = MUTABLE_FILES.iter().map(Path::new).collect::<Vec<_>>();
    Ok(support::plugin_fixture_with_mutable_files(&mutable_files)?)
}
