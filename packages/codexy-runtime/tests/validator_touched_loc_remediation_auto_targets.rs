use crate::support;

use std::process::{Command, Output};

use serde_json::Value;
use support::touched_loc::{fixture, regular_lines, regular_lines_from, stderr, validate, write};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn automatic_target_settings_are_omitted_and_rejected_as_a_matrix() -> TestResult {
    let targets = [
        ("autobins", "src/bin/ignored.rs", "src/bin/helper.rs"),
        ("autoexamples", "examples/ignored.rs", "examples/helper.rs"),
        ("autotests", "tests/ignored.rs", "tests/helper.rs"),
        ("autobenches", "benches/ignored.rs", "benches/helper.rs"),
    ];
    let repo = fixture(targets[0].1, regular_lines(252))?;
    write(
        repo.path(),
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nautobins = false\nautoexamples = false\nautotests = false\nautobenches = false\n",
    )?;
    write(repo.path(), "src/lib.rs", "")?;
    for (_, source, _) in targets.iter().skip(1) {
        write(repo.path(), source, &regular_lines(252))?;
    }
    amend(repo.path())?;
    assert_cargo_omits_automatic_targets(repo.path(), &targets)?;

    let rejected_targets = [targets[0], targets[1], targets[3]];
    for (_, source, helper) in rejected_targets {
        write(
            repo.path(),
            source,
            &format!("mod helper;\n{}", regular_lines(249)),
        )?;
        write(repo.path(), helper, &regular_lines_from(249, 3))?;
    }
    let output = validate(repo.path())?;
    assert!(!output.status.success(), "stderr:\n{}", stderr(&output));
    for (_, source, _) in rejected_targets {
        assert!(
            stderr(&output).contains(source),
            "missing disabled-target diagnostic for {source}\nstderr:\n{}",
            stderr(&output)
        );
    }
    Ok(())
}

fn assert_cargo_omits_automatic_targets(
    root: &std::path::Path,
    targets: &[(&str, &str, &str)],
) -> TestResult {
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
    for (_, source, _) in targets {
        assert!(
            metadata["packages"][0]["targets"]
                .as_array()
                .is_some_and(|cargo_targets| {
                    cargo_targets.iter().all(|target| {
                        !target["src_path"]
                            .as_str()
                            .is_some_and(|path| path.ends_with(source))
                    })
                }),
            "Cargo metadata unexpectedly retained {source}"
        );
    }
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
