use crate::support::{fixture_native_launcher, windows_static_python_fixture};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn windows_static_python_fixture_accepts_only_allowlisted_fail_closed_policy_pairs() -> TestResult {
    let temp = tempfile::tempdir()?;
    let shell = temp.path().join("codexy-repository-issue.sh");
    let command = temp.path().join("codexy-repository-issue.cmd");
    let python = temp.path().join("codexy-repository-issue.py");
    std::fs::write(&shell, "#!/bin/sh\n")?;
    std::fs::write(
        &command,
        "@echo off\nsetlocal EnableExtensions DisableDelayedExpansion\necho CODEXY_REPOSITORY_ISSUE_RUNTIME\n",
    )?;
    std::fs::write(&python, "#!/usr/bin/python3\n")?;
    assert_eq!(windows_static_python_fixture(&shell), Some(python));

    let unrelated = temp.path().join("unrelated.sh");
    std::fs::write(&unrelated, "#!/bin/sh\n")?;
    std::fs::write(unrelated.with_extension("cmd"), "@echo off\n")?;
    std::fs::write(unrelated.with_extension("py"), "#!/usr/bin/python3\n")?;
    assert_eq!(windows_static_python_fixture(&unrelated), None);
    Ok(())
}

#[test]
fn native_fixture_launcher_uses_only_the_platform_entrypoint() -> TestResult {
    let temp = tempfile::tempdir()?;
    let shell = temp.path().join("codexy-repository-issue.sh");
    let command = temp.path().join("codexy-repository-issue.cmd");
    std::fs::write(&shell, "#!/bin/sh\n")?;
    std::fs::write(&command, "@echo off\n")?;
    assert_eq!(fixture_native_launcher(false, &shell), Some(shell.clone()));
    assert_eq!(fixture_native_launcher(true, &shell), Some(command.clone()));
    std::fs::remove_file(command)?;
    assert_eq!(fixture_native_launcher(true, &shell), None);
    Ok(())
}
