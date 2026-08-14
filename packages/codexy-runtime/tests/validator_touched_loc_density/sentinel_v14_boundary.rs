use super::*;

#[test]
fn touched_loc_exposes_commands_inside_shell_substitutions() -> TestResult {
    let repo = fixture("scripts/check.sh", "exit 0\n".to_owned())?;
    write(
        repo.path(),
        "scripts/check.sh",
        "value=\"$(first; second; third)\"\n",
    )?;
    assert!(!validate(repo.path())?.status.success());
    Ok(())
}

#[test]
fn touched_loc_keeps_multiline_shell_quotes_masked() -> TestResult {
    let repo = fixture("scripts/check.sh", "exit 0\n".to_owned())?;
    write(
        repo.path(),
        "scripts/check.sh",
        "value=\"data\nfirst && second && third\n\"\n",
    )?;
    assert!(validate(repo.path())?.status.success());
    Ok(())
}

#[test]
fn touched_loc_queues_continuation_split_shell_heredocs() -> TestResult {
    let repo = fixture("scripts/check.sh", "exit 0\n".to_owned())?;
    write(
        repo.path(),
        "scripts/check.sh",
        "cat <<EO\\\nF\nfirst && second && third\nEOF\n",
    )?;
    assert!(validate(repo.path())?.status.success());
    Ok(())
}
