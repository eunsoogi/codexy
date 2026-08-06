use std::path::Path;
use std::process::{Command, Output};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn touched_loc_retains_reconciliation_after_main_advances() -> TestResult {
    let repo = tempfile::tempdir()?;
    init_repo(repo.path())?;
    write(repo.path(), "src/reconciled.rs", &multiline_source())?;
    commit(repo.path(), "initial stacked base")?;
    run(repo.path(), &["branch", "stacked"])?;

    write(repo.path(), "src/reconciled.rs", &collapsed_source("main"))?;
    commit(repo.path(), "main reduces reconciled file")?;
    track_origin_main(repo.path())?;

    run(repo.path(), &["switch", "stacked"])?;
    write(repo.path(), "src/parent.rs", "fn parent_change() {}\n")?;
    commit(repo.path(), "parent lane change")?;
    run(repo.path(), &["switch", "-c", "child"])?;
    run(
        repo.path(),
        &["merge", "--no-ff", "main", "-m", "reconcile current main"],
    )?;

    run(repo.path(), &["switch", "main"])?;
    write(repo.path(), "src/later.rs", "fn later_main_change() {}\n")?;
    commit(repo.path(), "main advances after reconciliation")?;
    track_origin_main(repo.path())?;
    run(repo.path(), &["switch", "child"])?;

    let output = validate(repo.path());
    assert!(
        output.status.success(),
        "historical reconciliation must survive a moving main tip\nstderr:\n{}",
        stderr(&output)
    );
    Ok(())
}

#[test]
fn touched_loc_uses_main_baseline_for_custom_resolution() -> TestResult {
    let repo = tempfile::tempdir()?;
    init_repo(repo.path())?;
    write(repo.path(), "src/conflicted.rs", "fn value() { base(); }\n")?;
    commit(repo.path(), "initial stacked base")?;
    run(repo.path(), &["branch", "stacked"])?;

    write(repo.path(), "src/conflicted.rs", &multiline_source())?;
    commit(repo.path(), "main expands conflicted file")?;
    track_origin_main(repo.path())?;

    run(repo.path(), &["switch", "stacked"])?;
    write(
        repo.path(),
        "src/conflicted.rs",
        "fn value() { parent(); }\n",
    )?;
    commit(repo.path(), "parent edits conflicted file")?;
    run(repo.path(), &["switch", "-c", "child"])?;
    let merge = Command::new("git")
        .args(["merge", "--no-ff", "main", "-m", "reconcile current main"])
        .current_dir(repo.path())
        .output()?;
    assert!(!merge.status.success(), "fixture must produce a conflict");
    write(
        repo.path(),
        "src/conflicted.rs",
        &collapsed_source("resolved"),
    )?;
    commit(repo.path(), "child resolves reconciliation conflict")?;

    let output = validate(repo.path());
    assert!(!output.status.success());
    assert!(stderr(&output).contains("multiline collapse"));
    Ok(())
}

fn init_repo(root: &Path) -> TestResult {
    run(root, &["init", "-q", "--initial-branch=main"])?;
    run(root, &["config", "user.email", "codexy@example.test"])?;
    run(root, &["config", "user.name", "Codexy Test"])
}

fn track_origin_main(root: &Path) -> TestResult {
    run(root, &["update-ref", "refs/remotes/origin/main", "HEAD"])
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
        "{}let summary = format!(\n    \"main\"\n);\n",
        regular_lines(249)
    )
}

fn collapsed_source(value: &str) -> String {
    format!(
        "{}let summary = format!(\"{value}\");\n",
        regular_lines(249)
    )
}

fn regular_lines(count: usize) -> String {
    (0..count)
        .map(|index| format!("fn line_{index}() {{}}\n"))
        .collect()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
