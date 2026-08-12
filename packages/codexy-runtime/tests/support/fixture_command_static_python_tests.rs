use crate::support::windows_static_python_fixture;

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
