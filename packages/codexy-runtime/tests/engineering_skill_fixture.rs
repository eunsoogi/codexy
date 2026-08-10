use std::path::{Path, PathBuf};

use crate::support::{self, TestResult};

const MUTABLE_FILES: &[&str] = &[
    "skills/engineering/SKILL.md",
    "skills/engineering/references/diagnosis.md",
    "skills/engineering/references/legacy-rule-manifest.json",
    "skills/engineering/references/legacy-rule-mappings/debugging.json",
    "skills/engineering/references/test-driven-development.md",
];

#[test]
fn declared_engineering_mutation_targets_replace_stale_readonly_files() -> TestResult {
    let temporary = tempfile::tempdir()?;
    let source = temporary.path().join("source");
    let target = temporary.path().join("target");
    let destination = Path::new("skills/engineering/references/test-driven-development.md");
    let undeclared = Path::new("skills/engineering/references/immutable.md");
    for (path, contents) in [(destination, "authoritative\n"), (undeclared, "immutable\n")] {
        let source_path = source.join(path);
        std::fs::create_dir_all(source_path.parent().expect("source parent"))?;
        std::fs::write(source_path, contents)?;
    }
    let stale_target = target.join(destination);
    std::fs::create_dir_all(stale_target.parent().expect("target parent"))?;
    std::fs::write(&stale_target, "stale\n")?;
    let mut permissions = std::fs::metadata(&stale_target)?.permissions();
    permissions.set_readonly(true);
    std::fs::set_permissions(&stale_target, permissions)?;
    support::plugin_fixture_copy::make_seed_readonly(&source)?;

    let paths = MUTABLE_FILES.iter().map(Path::new).collect::<Vec<_>>();
    support::plugin_fixture_copy::materialize_seed(
        &source,
        &target,
        Path::new(""),
        &paths,
        None,
    )?;

    assert_eq!(std::fs::read_to_string(&stale_target)?, "authoritative\n");
    assert!(!std::fs::metadata(&stale_target)?.permissions().readonly());
    assert!(std::fs::metadata(target.join(undeclared))?.permissions().readonly());
    Ok(())
}

pub(super) fn copy_engineering_skill_fixture() -> TestResult<(tempfile::TempDir, PathBuf)> {
    let paths = MUTABLE_FILES.iter().map(Path::new).collect::<Vec<_>>();
    Ok(support::copy_plugin_fixture_with_mutable_files(&paths)?)
}
