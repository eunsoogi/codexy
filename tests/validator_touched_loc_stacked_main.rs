use std::path::Path;
use std::process::{Command, Output};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn touched_loc_ignores_reconciled_main_files_but_checks_child_changes() -> TestResult {
    let repo = tempfile::tempdir()?;
    init_repo(repo.path())?;
    write(repo.path(), "src/reconciled.rs", &multiline_source())?;
    commit(repo.path(), "initial stacked base")?;
    run(repo.path(), &["branch", "stacked"])?;

    write(repo.path(), "src/reconciled.rs", &collapsed_source())?;
    commit(repo.path(), "reconcile main")?;
    run(
        repo.path(),
        &["update-ref", "refs/remotes/origin/main", "HEAD"],
    )?;

    run(repo.path(), &["switch", "stacked"])?;
    write(repo.path(), "src/parent.rs", &regular_lines(251))?;
    commit(repo.path(), "parent lane change")?;

    run(repo.path(), &["switch", "-c", "child", "stacked"])?;
    run(
        repo.path(),
        &["merge", "--no-ff", "main", "-m", "reconcile current main"],
    )?;
    write(repo.path(), "src/child.rs", "fn child_change() {}\n")?;
    commit(repo.path(), "child change")?;

    let reconciled = validate(repo.path());
    assert!(
        reconciled.status.success(),
        "current-main reconciliation must not be treated as a child LOC remediation\nstderr:\n{}",
        stderr(&reconciled)
    );

    write(repo.path(), "src/child.rs", &regular_lines(251))?;
    let oversized = validate(repo.path());
    assert!(!oversized.status.success());
    assert!(stderr(&oversized).contains("src/child.rs has 251 lines"));
    Ok(())
}

fn init_repo(root: &Path) -> TestResult {
    run(root, &["init", "-q", "--initial-branch=main"])?;
    run(root, &["config", "user.email", "codexy@example.test"])?;
    run(root, &["config", "user.name", "Codexy Test"])
}

fn write(root: &Path, path: &str, text: &str) -> std::io::Result<()> {
    let path = root.join(path);
    std::fs::create_dir_all(path.parent().expect("fixture path has a parent"))?;
    std::fs::write(path, text)
}

fn commit(root: &Path, message: &str) -> TestResult {
    run(root, &["add", "."])?;
    run(root, &["commit", "-qm", message])
}

fn validate(root: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args(["--check-touched-loc", "--base-ref", "stacked"])
        .current_dir(root)
        .output()
        .expect("validator command should run")
}

fn run(root: &Path, args: &[&str]) -> TestResult {
    let output = Command::new("git").args(args).current_dir(root).output()?;
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        stderr(&output)
    );
    Ok(())
}

fn multiline_source() -> String {
    format!(
        "{}let summary = format!(\n    \"status\"\n);\n",
        regular_lines(249)
    )
}

fn collapsed_source() -> String {
    format!("{}let summary = format!(\"status\");\n", regular_lines(249))
}

fn regular_lines(count: usize) -> String {
    (0..count)
        .map(|index| format!("fn line_{index}() {{}}\n"))
        .collect()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
