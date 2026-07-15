mod support;

use std::process::{Command, Output};

use serde_json::Value;
use support::touched_loc::{fixture, regular_lines, regular_lines_from, stderr, validate, write};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn touched_loc_rejects_roots_disabled_by_cargo_automatic_target_settings() -> TestResult {
    for (setting, source, helper) in [
        ("autobins", "src/bin/ignored.rs", "src/bin/helper.rs"),
        ("autoexamples", "examples/ignored.rs", "examples/helper.rs"),
        ("autobenches", "benches/ignored.rs", "benches/helper.rs"),
    ] {
        let repo = fixture(source, regular_lines(252))?;
        write(
            repo.path(),
            "Cargo.toml",
            &format!("[package]\nname = \"app\"\n{setting} = false\n"),
        )?;
        write(repo.path(), "src/lib.rs", "")?;
        amend(repo.path())?;
        assert_cargo_omits_automatic_target(repo.path(), source)?;
        write(
            repo.path(),
            source,
            &format!("mod helper;\n{}", regular_lines(249)),
        )?;
        write(repo.path(), helper, &regular_lines_from(249, 3))?;

        let output = validate(repo.path())?;
        assert!(
            !output.status.success(),
            "setting: {setting}\nstderr:\n{}",
            stderr(&output)
        );
    }
    Ok(())
}

#[test]
fn cargo_metadata_omits_each_disabled_automatic_target_kind() -> TestResult {
    for (setting, source) in [
        ("autobins", "src/bin/ignored.rs"),
        ("autoexamples", "examples/ignored.rs"),
        ("autotests", "tests/ignored.rs"),
        ("autobenches", "benches/ignored.rs"),
    ] {
        let repo = fixture(source, String::new())?;
        write(
            repo.path(),
            "Cargo.toml",
            &format!("[package]\nname = \"app\"\n{setting} = false\n"),
        )?;
        write(repo.path(), "src/lib.rs", "")?;
        amend(repo.path())?;
        assert_cargo_omits_automatic_target(repo.path(), source)?;
    }
    Ok(())
}

fn assert_cargo_omits_automatic_target(root: &std::path::Path, source: &str) -> TestResult {
    let metadata = run(
        root,
        "cargo",
        &[
            "metadata",
            "--offline",
            "--no-deps",
            "--format-version",
            "1",
        ],
    )?;
    assert!(
        metadata.status.success(),
        "cargo metadata stderr:\n{}",
        stderr(&metadata)
    );
    let metadata: Value = serde_json::from_slice(&metadata.stdout)?;
    assert!(
        metadata["packages"][0]["targets"]
            .as_array()
            .is_some_and(|targets| {
                targets.iter().all(|target| {
                    !target["src_path"]
                        .as_str()
                        .is_some_and(|path| path.ends_with(source))
                })
            })
    );
    Ok(())
}

fn amend(root: &std::path::Path) -> TestResult {
    let output = run(root, "git", &["add", "."])?;
    assert!(
        output.status.success(),
        "git add stderr:\n{}",
        stderr(&output)
    );
    let output = run(root, "git", &["commit", "--amend", "--no-edit", "-q"])?;
    assert!(
        output.status.success(),
        "git amend stderr:\n{}",
        stderr(&output)
    );
    Ok(())
}

fn run(root: &std::path::Path, program: &str, args: &[&str]) -> std::io::Result<Output> {
    Command::new(program).args(args).current_dir(root).output()
}
