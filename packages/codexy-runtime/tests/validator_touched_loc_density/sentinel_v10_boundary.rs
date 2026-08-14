use super::*;

#[test]
fn touched_loc_keeps_python_escaped_triple_closes_masked() -> TestResult {
    let repo = fixture("scripts/check.py", "pass\n".to_owned())?;
    write(
        repo.path(),
        "scripts/check.py",
        "data = \"\"\"\nescaped = \\\u{22}\"\"\nfirst(); second(); third();\n\"\"\"\n",
    )?;
    assert!(validate(repo.path())?.status.success());
    Ok(())
}

#[test]
fn touched_loc_distinguishes_shell_heredoc_terminator_modes() -> TestResult {
    let repo = fixture("scripts/release.sh", "exit 0\n".to_owned())?;
    write(
        repo.path(),
        "scripts/release.sh",
        "cat <<EOF\n  EOF\nfirst && second && third\nEOF\n",
    )?;
    assert!(validate(repo.path())?.status.success());
    write(
        repo.path(),
        "scripts/release.sh",
        "cat <<-EOF\n\tEOF\nfirst && second && third\n",
    )?;
    assert!(!validate(repo.path())?.status.success());
    write(
        repo.path(),
        "src/check.js",
        "const label = `${`${(() => { first(); second(); third(); })()}`}`;\n",
    )?;
    assert!(!validate(repo.path())?.status.success());
    Ok(())
}

#[test]
fn touched_loc_exposes_javascript_template_interpolation() -> TestResult {
    let repo = fixture("src/check.js", "const ready = true;\n".to_owned())?;
    write(repo.path(), "src/check.js", "const label = `${value}`;\n")?;
    assert!(validate(repo.path())?.status.success());
    write(
        repo.path(),
        "src/check.js",
        "const label = `${(() => { first(); second(); third(); })()}`;\n",
    )?;
    assert!(!validate(repo.path())?.status.success());
    Ok(())
}
