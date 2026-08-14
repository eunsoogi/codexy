use super::*;

#[test]
fn touched_loc_exposes_nested_shell_substitutions_inside_quotes() -> TestResult {
    let repo = fixture("scripts/check.sh", "exit 0\n".to_owned())?;
    write(
        repo.path(),
        "scripts/check.sh",
        "value=\"$(echo \"$(first; second; third)\")\"\n",
    )?;
    assert!(!validate(repo.path())?.status.success());
    Ok(())
}

#[test]
fn touched_loc_does_not_escape_single_quote_delimiters() -> TestResult {
    let repo = fixture("scripts/check.sh", "exit 0\n".to_owned())?;
    write(
        repo.path(),
        "scripts/check.sh",
        "value='x\\'; first; second; third\n",
    )?;
    assert!(!validate(repo.path())?.status.success());
    Ok(())
}

#[test]
fn touched_loc_queues_heredocs_after_multiline_quote_closures() -> TestResult {
    let repo = fixture("scripts/check.sh", "exit 0\n".to_owned())?;
    write(
        repo.path(),
        "scripts/check.sh",
        "value=\"data\n\"; cat <<EOF\nfirst && second && third\nEOF\n",
    )?;
    assert!(validate(repo.path())?.status.success());
    Ok(())
}
