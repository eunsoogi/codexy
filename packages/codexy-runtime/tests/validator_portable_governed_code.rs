use std::{fs, process::Command};

use crate::support::TestResult;

#[test]
fn package_owned_checker_is_explicit_path_only_and_enforces_250_lines() -> TestResult {
    let root = codexy_runtime::paths::repository_root().join("plugins/codexy");
    let policy = root.join("skills/engineering/references/governed-code-policy.json");
    let checker = root.join("skills/engineering/scripts/check_governed_code.py");
    assert!(policy.is_file(), "missing package-owned LOC policy");
    assert!(checker.is_file(), "missing package-owned LOC checker");

    let temp = tempfile::tempdir()?;
    let exactly_250 = temp.path().join("exactly-250.py");
    let over_limit = temp.path().join("over-limit.py");
    fs::write(&exactly_250, (1..=250).map(|n| format!("line_{n}\n")).collect::<String>())?;
    fs::write(&over_limit, (1..=251).map(|n| format!("line_{n}\n")).collect::<String>())?;

    let valid = run(&checker, &exactly_250)?;
    assert!(valid.status.success(), "250-line file failed: {valid:?}");
    let invalid = run(&checker, &over_limit)?;
    assert!(!invalid.status.success(), "251-line file passed");

    let hostile = temp.path().join("scripts/validate-plugin-config.sh");
    fs::create_dir_all(hostile.parent().ok_or("hostile parent")?)?;
    fs::write(&hostile, "exit 99\n")?;
    let from_hostile_cwd = Command::new("python3")
        .arg(&checker)
        .arg("--path")
        .arg(&exactly_250)
        .current_dir(temp.path())
        .output()?;
    assert!(from_hostile_cwd.status.success());
    Ok(())
}

fn run(checker: &std::path::Path, path: &std::path::Path) -> TestResult<std::process::Output> {
    Ok(Command::new("python3")
        .arg(checker)
        .arg("--path")
        .arg(path)
        .output()?)
}
