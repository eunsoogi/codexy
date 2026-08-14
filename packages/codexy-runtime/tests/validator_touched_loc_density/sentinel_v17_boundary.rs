use super::*;

#[test]
fn touched_loc_exposes_literal_shell_hash_suffixes() -> TestResult {
    let repo = fixture("scripts/check.sh", "exit 0\n".to_owned())?;
    write(
        repo.path(),
        "scripts/check.sh",
        "echo foo#bar; first; second; third\n",
    )?;
    assert!(!validate(repo.path())?.status.success());
    Ok(())
}

#[test]
fn touched_loc_exposes_escaped_shell_hash_suffixes() -> TestResult {
    let repo = fixture("scripts/check.sh", "exit 0\n".to_owned())?;
    write(
        repo.path(),
        "scripts/check.sh",
        "echo \\ #bar; first; second; third\n",
    )?;
    assert!(!validate(repo.path())?.status.success());
    Ok(())
}

#[test]
fn touched_loc_exposes_nested_shell_hash_suffixes() -> TestResult {
    let repo = fixture("scripts/check.sh", "exit 0\n".to_owned())?;
    write(
        repo.path(),
        "scripts/check.sh",
        "value=$(echo foo#bar; first; second; third)\n",
    )?;
    assert!(!validate(repo.path())?.status.success());
    Ok(())
}

#[test]
fn touched_loc_keeps_shell_comment_payloads_masked() -> TestResult {
    let repo = fixture("scripts/check.sh", "exit 0\n".to_owned())?;
    write(
        repo.path(),
        "scripts/check.sh",
        "# first && second && third\n",
    )?;
    assert!(validate(repo.path())?.status.success());
    Ok(())
}
