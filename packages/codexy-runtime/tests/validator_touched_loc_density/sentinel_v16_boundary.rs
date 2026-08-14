use super::*;

#[test]
fn touched_loc_exposes_code_after_root_escaped_quote_heredocs() -> TestResult {
    let repo = fixture("scripts/check.sh", "exit 0\n".to_owned())?;
    write(
        repo.path(),
        "scripts/check.sh",
        "echo \\\"; cat <<EOF\npayload\nEOF\nfirst && second && third\n",
    )?;
    assert!(!validate(repo.path())?.status.success());
    Ok(())
}

#[test]
fn touched_loc_exposes_code_after_nested_escaped_quote_heredocs() -> TestResult {
    let repo = fixture("scripts/check.sh", "exit 0\n".to_owned())?;
    write(
        repo.path(),
        "scripts/check.sh",
        "value=\"$(echo \\\"; cat <<EOF\npayload\nEOF\nfirst && second && third\n)\"\n",
    )?;
    assert!(!validate(repo.path())?.status.success());
    Ok(())
}

#[test]
fn touched_loc_keeps_quoted_arithmetic_shifts_out_of_heredoc_admission() -> TestResult {
    let repo = fixture("scripts/check.sh", "exit 0\n".to_owned())?;
    write(
        repo.path(),
        "scripts/check.sh",
        "value=$(( marker = \"))\" << 2 ))\nfirst && second && third\n",
    )?;
    assert!(!validate(repo.path())?.status.success());
    Ok(())
}
