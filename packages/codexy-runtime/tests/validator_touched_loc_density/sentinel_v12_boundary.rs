use super::*;

#[test]
fn touched_loc_exposes_template_expressions_after_comment_and_regex_braces() -> TestResult {
    let comment = fixture("src/check.js", "const ready = true;\n".to_owned())?;
    write(
        comment.path(),
        "src/check.js",
        r#"const label = `${/* } */ (() => { first(); second(); third(); })()}`;
"#,
    )?;
    assert!(!validate(comment.path())?.status.success());

    let regex = fixture("src/check.js", "const ready = true;\n".to_owned())?;
    write(
        regex.path(),
        "src/check.js",
        r#"const label = `${/}/.test(value); (() => { first(); second(); third(); })()}`;
"#,
    )?;
    assert!(!validate(regex.path())?.status.success());
    Ok(())
}

#[test]
fn touched_loc_closes_templates_after_line_continuations() -> TestResult {
    let repo = fixture("src/check.js", "const ready = true;\n".to_owned())?;
    write(
        repo.path(),
        "src/check.js",
        r#"const label = `body \
`;
first(); second(); third();
"#,
    )?;
    assert!(!validate(repo.path())?.status.success());
    Ok(())
}

#[test]
fn touched_loc_distinguishes_arithmetic_commands_and_queues_heredocs() -> TestResult {
    let arithmetic = fixture("scripts/check.sh", "exit 0\n".to_owned())?;
    write(
        arithmetic.path(),
        "scripts/check.sh",
        "(( value = 1 << 2 ))\nfirst && second && third\n",
    )?;
    assert!(!validate(arithmetic.path())?.status.success());

    let quoted = fixture("scripts/check.sh", "exit 0\n".to_owned())?;
    write(
        quoted.path(),
        "scripts/check.sh",
        "cat <<\"E\\\"OF\"\npayload\nE\"OF\nfirst && second && third\n",
    )?;
    assert!(!validate(quoted.path())?.status.success());

    let queued = fixture("scripts/check.sh", "exit 0\n".to_owned())?;
    write(
        queued.path(),
        "scripts/check.sh",
        "cat <<A <<B\nfirst && second && third\nA\nsecond payload\nB\n",
    )?;
    assert!(validate(queued.path())?.status.success());
    Ok(())
}
