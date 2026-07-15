use std::path::Path;
use std::process::{Command, Output};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn touched_loc_rejects_collapse_with_unrelated_added_file() -> TestResult {
    let repo = fixture("src/too_large.rs", multiline_source())?;
    std::fs::write(
        repo.path().join("src/too_large.rs"),
        format!(
            "{}let renamed_summary = format!(\"status\");\n",
            regular_lines(249)
        ),
    )?;
    write(repo.path(), "src/unrelated.rs", "fn unrelated() {}\n")?;
    let output = validate(repo.path())?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("multiline collapse"));
    Ok(())
}

#[test]
fn touched_loc_allows_extraction_into_existing_module() -> TestResult {
    for (path, module, helper) in [
        ("src/too_large.rs", "helper", "src/too_large/helper.rs"),
        ("src/foo.rs", "helper", "src/foo/helper/mod.rs"),
        ("tests/foo.rs", "common", "tests/common.rs"),
    ] {
        let repo = fixture(path, format!("mod {module};\n{}", multiline_source()))?;
        write(repo.path(), helper, "fn existing() {}\n")?;
        run(repo.path(), &["add", "."])?;
        run(repo.path(), &["commit", "-qm", "existing helper"])?;
        write(
            repo.path(),
            helper,
            "fn existing() {}\nlet summary = format!(\"status\");\n",
        )?;
        std::fs::write(
            repo.path().join(path),
            format!("mod {module};\n{}", regular_lines(249)),
        )?;

        let output = validate(repo.path())?;

        assert!(
            output.status.success(),
            "{path} stderr:\n{}",
            stderr(&output)
        );
    }
    Ok(())
}

#[test]
fn touched_loc_rejects_unrelated_existing_module_declaration() -> TestResult {
    let repo = fixture("src/too_large.rs", multiline_source())?;
    write(repo.path(), "src/helper.rs", "fn existing() {}\n")?;
    run(repo.path(), &["add", "."])?;
    run(repo.path(), &["commit", "-qm", "existing helper"])?;
    write(
        repo.path(),
        "src/helper.rs",
        "fn existing() {}\nfn unrelated_change() {}\n",
    )?;
    std::fs::write(
        repo.path().join("src/too_large.rs"),
        format!(
            "mod helper;\n{}let renamed_summary = format!(\"status\");\n",
            regular_lines(248)
        ),
    )?;
    let output = validate(repo.path())?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("multiline collapse"));
    Ok(())
}

#[test]
fn touched_loc_allows_visible_module_extraction() -> TestResult {
    for declaration in ["pub mod helper;", "pub(crate) mod helper;"] {
        let repo = fixture("src/too_large.rs", multiline_source())?;
        write(
            repo.path(),
            "src/too_large/helper.rs",
            "let summary = format!(\"status\");\n",
        )?;
        std::fs::write(
            repo.path().join("src/too_large.rs"),
            format!("{declaration}\n{}", regular_lines(249)),
        )?;
        let output = validate(repo.path())?;
        assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    }
    Ok(())
}

#[test]
fn touched_loc_rejects_unrelated_visible_module_change() -> TestResult {
    let repo = fixture("src/too_large.rs", multiline_source())?;
    write(repo.path(), "src/helper.rs", "fn unrelated() {}\n")?;
    std::fs::write(
        repo.path().join("src/too_large.rs"),
        format!(
            "pub mod helper;\n{}let renamed_summary = format!(\"status\");\n",
            regular_lines(248)
        ),
    )?;
    let output = validate(repo.path())?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("multiline collapse"));
    Ok(())
}

#[test]
fn touched_loc_allows_normal_duplicate_reduction() -> TestResult {
    let repo = fixture(
        "src/too_large.rs",
        format!(
            "{}fn duplicate() {{}}\nfn duplicate() {{}}\nfn duplicate() {{}}\n",
            regular_lines(249)
        ),
    )?;
    std::fs::write(
        repo.path().join("src/too_large.rs"),
        format!("{}fn duplicate() {{}}\n", regular_lines(249)),
    )?;
    let output = validate(repo.path())?;
    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    Ok(())
}

#[test]
fn touched_loc_rejects_collapse_with_incidental_duplicate_drop() -> TestResult {
    let repo = fixture(
        "src/too_large.rs",
        format!(
            "{}fn first() {{\n    call(\n    );\n}}\nfn second() {{\n    other(\n    );\n}}\n",
            regular_lines(245)
        ),
    )?;
    std::fs::write(
        repo.path().join("src/too_large.rs"),
        format!(
            "{}fn renamed_first() {{ call(); }}\nfn second() {{\n    other(\n    );\n}}\n",
            regular_lines(245)
        ),
    )?;
    let output = validate(repo.path())?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("multiline collapse"));
    Ok(())
}

#[test]
fn touched_loc_allows_standalone_test_target_split() -> TestResult {
    let repo = fixture("tests/too_large.rs", regular_lines(252))?;
    std::fs::write(repo.path().join("tests/too_large.rs"), regular_lines(249))?;
    write(
        repo.path(),
        "tests/extracted_cases.rs",
        &regular_lines_from(249, 3),
    )?;
    let output = validate(repo.path())?;
    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    Ok(())
}

#[test]
fn touched_loc_rejects_incidental_test_target_overlap() -> TestResult {
    let repo = fixture("tests/too_large.rs", regular_lines(252))?;
    std::fs::write(repo.path().join("tests/too_large.rs"), regular_lines(249))?;
    write(repo.path(), "tests/unrelated.rs", "fn line_249() {}\n")?;
    let output = validate(repo.path())?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("multiline collapse"));
    Ok(())
}

fn fixture(path: &str, source: String) -> TestResult<tempfile::TempDir> {
    let repo = tempfile::tempdir()?;
    run(repo.path(), &["init", "-q"])?;
    run(
        repo.path(),
        &["config", "user.email", "codexy@example.test"],
    )?;
    run(repo.path(), &["config", "user.name", "Codexy Test"])?;
    write(repo.path(), path, &source)?;
    run(repo.path(), &["add", "."])?;
    run(repo.path(), &["commit", "-qm", "initial"])?;
    Ok(repo)
}

fn write(root: &Path, path: &str, text: &str) -> std::io::Result<()> {
    let path = root.join(path);
    std::fs::create_dir_all(path.parent().expect("fixture file parent"))?;
    std::fs::write(path, text)
}

fn validate(root: &Path) -> TestResult<Output> {
    Ok(Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args(["--check-touched-loc", "--base-ref", "HEAD"])
        .current_dir(root)
        .output()?)
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

fn regular_lines(count: usize) -> String {
    (0..count)
        .map(|index| format!("fn line_{index}() {{}}\n"))
        .collect()
}

fn regular_lines_from(start: usize, count: usize) -> String {
    (start..start + count)
        .map(|index| format!("fn line_{index}() {{}}\n"))
        .collect()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
