use super::*;

#[test]
fn touched_loc_tracks_successive_python_triple_string_spans() -> TestResult {
    let repo = fixture("scripts/check.py", "pass\n".to_owned())?;
    write(
        repo.path(),
        "scripts/check.py",
        "data = \"\"\"inline\"\"\" + \"\"\"\nfirst(); second(); third();\n\"\"\"\n",
    )?;
    assert!(validate(repo.path())?.status.success());
    Ok(())
}

#[test]
fn touched_loc_keeps_python_code_after_closed_inline_span() -> TestResult {
    let repo = fixture("scripts/check.py", "pass\n".to_owned())?;
    write(
        repo.path(),
        "scripts/check.py",
        "data = \"\"\"inline\"\"\"; first(); second(); third()\n",
    )?;
    assert!(!validate(repo.path())?.status.success());
    Ok(())
}
