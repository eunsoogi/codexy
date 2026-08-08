use super::{TestResult, regular_lines};
use crate::support::touched_loc::fixture;

#[test]
fn touched_loc_ignores_deleted_governed_files() -> TestResult {
    let repo = fixture("src/base.rs", "fn base() {}\n".to_owned())?;
    run(repo.path(), &["branch", "stacked"])?;
    std::fs::create_dir_all(repo.path().join("tests"))?;
    std::fs::write(
        repo.path().join("tests/legacy_admission.rs"),
        regular_lines(251),
    )?;
    run(repo.path(), &["add", "tests/legacy_admission.rs"])?;
    run(repo.path(), &["commit", "-qm", "add legacy admission test"])?;
    std::fs::remove_file(repo.path().join("tests/legacy_admission.rs"))?;
    let diagnostics = codexy_runtime::validation::touched_loc_diagnostics(repo.path(), "stacked")?;
    assert!(diagnostics.is_empty(), "diagnostics: {diagnostics:?}");
    Ok(())
}

fn run(root: &std::path::Path, args: &[&str]) -> TestResult {
    let output = std::process::Command::new("git").args(args).current_dir(root).output()?;
    assert!(output.status.success(), "git {args:?} failed");
    Ok(())
}
