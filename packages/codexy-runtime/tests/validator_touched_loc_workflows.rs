use crate::support;

use support::touched_loc::{fixture, regular_lines, stderr, validate, write};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn touched_loc_governs_workflow_yaml_only() -> TestResult {
    for path in [
        ".github/workflows/oversized.yml",
        ".github/workflows/oversized.yaml",
    ] {
        let repo = fixture(path, regular_lines(251))?;
        let output = validate(repo.path())?;
        assert!(!output.status.success(), "{path} escaped governance");
        assert!(stderr(&output).contains(&format!("{path} has 251 lines")));
    }
    for path in ["config/oversized.yml", "fixtures/oversized.yaml"] {
        let repo = fixture(path, regular_lines(251))?;
        let output = validate(repo.path())?;
        assert!(output.status.success(), "{path}: {}", stderr(&output));
    }
    let repo = fixture(".github/workflows/boundary.yml", regular_lines(250))?;
    let output = validate(repo.path())?;
    assert!(output.status.success(), "boundary: {}", stderr(&output));
    Ok(())
}

#[test]
fn touched_loc_accepts_cohesive_workflow_script_extraction() -> TestResult {
    let baseline = format!(
        "name: fixture\njobs:\n  release:\n    steps:\n      - run: |\n{}",
        regular_lines(247)
    );
    let repo = fixture(".github/workflows/release.yml", baseline)?;
    write(
        repo.path(),
        ".github/workflows/release.yml",
        "name: fixture\njobs:\n  release:\n    steps:\n      - run: scripts/reconcile-release\n",
    )?;
    write(repo.path(), "scripts/reconcile-release", &regular_lines(247))?;
    let output = validate(repo.path())?;
    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    Ok(())
}

#[test]
fn touched_loc_parses_only_safe_single_script_commands() -> TestResult {
    let repo = fixture(".github/workflows/release.yml", workflow_baseline())?;
    let root = repo.path();
    assert_workflow_extraction_with_script(
        root,
        r#"scripts/validate-plugin-config.sh --check-touched-loc --base-ref "origin/${{ github.base_ref }}""#,
        "scripts/validate-plugin-config.sh",
        true,
    )?;
    assert_multiline_workflow_extraction(
        root,
        "scripts/reconcile-release --message release\nbuild",
        false,
    )?;
    for command in [
        "scripts/reconcile-release --check",
        "scripts/reconcile-release --mode=check release",
        "scripts/reconcile-release --message \"release build\"",
        "scripts/reconcile-release --message 'release build'",
        "scripts/reconcile-release --check --tag \"$RELEASE_TAG\"",
        "scripts/reconcile-release --tag $RELEASE_TAG",
        "scripts/reconcile-release --tag ${RELEASE_TAG}",
        "scripts/reconcile-release --tag \"${RELEASE_TAG}\"",
        r#"scripts/reconcile-release --artifact "release-${{ github.run_id }}""#,
    ] {
        assert_workflow_extraction(root, command, true)?;
    }
    for command in [
        "\"scripts/reconcile-release\" --check",
        "command scripts/reconcile-release --check",
        "MODE=check scripts/reconcile-release",
        "scripts/reconcile-release > result.txt",
        "scripts/reconcile-release $(echo --check)",
        "scripts/reconcile-release `echo --check`",
        "scripts/reconcile-release $",
        "scripts/reconcile-release ${}",
        "scripts/reconcile-release ${RELEASE-TAG}",
        "scripts/reconcile-release $9",
        "scripts/reconcile-release \"$RELEASE_TAG",
        "scripts/reconcile-release prefix$RELEASE_TAG",
        "scripts/reconcile-release prefix\"$RELEASE_TAG\"",
        "scripts/reconcile-release $RELEASE_TAG/suffix",
        "scripts/reconcile-release ${RELEASE_TAG:-latest}",
        r#"scripts/reconcile-release --base ${{ github.base_ref }}"#,
        r#"scripts/reconcile-release --base "origin/${{ github.base_ref }""#,
        r#"scripts/reconcile-release --base "origin/${{ }}""#,
        r#"scripts/reconcile-release --base "origin/${{ github. base_ref }}""#,
        r#"scripts/reconcile-release --base "origin/${{ github.${{ base_ref }} }}""#,
        "scripts/reconcile-release --message \"release build",
        "scripts/reconcile-release --message 'release build",
        r#"scripts/reconcile-release --message "release\ build""#,
        "scripts/reconcile-release --message release\\ build",
        "scripts/reconcile-release *.tgz",
        "scripts/reconcile-release <(echo --check)",
        "scripts/reconcile-release < input.txt",
        "scripts/reconcile-release || echo failed",
        "scripts/reconcile-release | tee result.txt",
        "scripts/reconcile-release|tee result.txt",
        "scripts/reconcile-release && echo done",
        "scripts/reconcile-release&&echo done",
        "scripts/reconcile-release; echo done",
        "scripts/reconcile-release;echo done",
        "/scripts/reconcile-release --check",
        "scripts/../reconcile-release --check",
        "cargo run --bin reconcile-release",
    ] {
        assert_workflow_extraction(root, command, false)?;
    }
    Ok(())
}

#[test]
fn touched_loc_fixtures_keep_private_histories_when_reusing_git_metadata() -> TestResult {
    let first = fixture("src/lib.rs", "pub fn first() {}\n".to_owned())?;
    let second = fixture("src/lib.rs", "pub fn second() {}\n".to_owned())?;
    write(first.path(), "src/lib.rs", "pub fn mutated() {}\n")?;
    let first_add = std::process::Command::new("git")
        .args(["add", "src/lib.rs"])
        .current_dir(first.path())
        .output()?;
    assert!(first_add.status.success());
    let first_commit = std::process::Command::new("git")
        .args(["commit", "-qm", "mutated"])
        .current_dir(first.path())
        .output()?;
    assert!(first_commit.status.success());

    let second_head = std::process::Command::new("git")
        .args(["show", "HEAD:src/lib.rs"])
        .current_dir(second.path())
        .output()?;
    assert!(second_head.status.success());
    assert_eq!(
        String::from_utf8(second_head.stdout)?,
        "pub fn second() {}\n",
        "fixture history must remain private to its own temporary repository"
    );

    let helper = std::fs::read_to_string(
        codexy_runtime::paths::runtime_package_root().join("tests/support/touched_loc.rs"),
    )?;
    support::assert_structured_literals(
        &helper,
        "private touched-LOC Git metadata seed",
        &["fn git_fixture_seed", "copy_dir(&seed, &repo.path().join(\".git\"))"],
    );
    Ok(())
}

fn assert_multiline_workflow_extraction(
    root: &std::path::Path,
    command: &str,
    accepted: bool,
) -> TestResult {
    let indented = command.replace('\n', "\n          ");
    write(
        root,
        ".github/workflows/release.yml",
        &format!(
            "name: fixture\njobs:\n  release:\n    steps:\n      - run: |\n          {indented}\n"
        ),
    )?;
    write(root, "scripts/reconcile-release", &regular_lines(247))?;
    let output = validate(root)?;
    assert_eq!(output.status.success(), accepted, "{command}: {}", stderr(&output));
    Ok(())
}

fn assert_workflow_extraction(
    root: &std::path::Path,
    command: &str,
    accepted: bool,
) -> TestResult {
    assert_workflow_extraction_with_script(root, command, "scripts/reconcile-release", accepted)
}

fn assert_workflow_extraction_with_script(
    root: &std::path::Path,
    command: &str,
    extracted_script: &str,
    accepted: bool,
) -> TestResult {
    write(
        root,
        ".github/workflows/release.yml",
        &format!("name: fixture\njobs:\n  release:\n    steps:\n      - run: {command}\n"),
    )?;
    write(root, extracted_script, &regular_lines(247))?;
    let output = validate(root)?;
    assert_eq!(output.status.success(), accepted, "{command}: {}", stderr(&output));
    Ok(())
}

fn workflow_baseline() -> String {
    format!(
        "name: fixture\njobs:\n  release:\n    steps:\n      - run: |\n{}",
        regular_lines(247)
    )
}
