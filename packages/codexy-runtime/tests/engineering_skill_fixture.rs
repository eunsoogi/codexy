use std::path::{Path, PathBuf};

use crate::support::{self, TestResult};

const MUTABLE_FILES: &[&str] = &[
    "skills/engineering/SKILL.md",
    "skills/engineering/references/diagnosis.md",
    "skills/engineering/references/legacy-rule-manifest.json",
    "skills/engineering/references/legacy-rule-mappings/debugging.json",
];

pub(super) fn copy_engineering_skill_fixture() -> TestResult<(tempfile::TempDir, PathBuf)> {
    let paths = MUTABLE_FILES.iter().map(Path::new).collect::<Vec<_>>();
    Ok(support::copy_plugin_fixture_with_mutable_files(&paths)?)
}
