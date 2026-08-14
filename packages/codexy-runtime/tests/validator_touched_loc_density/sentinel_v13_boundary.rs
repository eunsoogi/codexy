use super::*;

#[test]
fn touched_loc_closes_interpolation_strings_after_line_continuations() -> TestResult {
    let after = fixture("src/check.js", "const ready = true;\n".to_owned())?;
    write(
        after.path(),
        "src/check.js",
        r#"const label = `${"body \
"}`;
first(); second(); third();
"#,
    )?;
    assert!(!validate(after.path())?.status.success());

    let string = fixture("src/check.js", "const ready = true;\n".to_owned())?;
    write(
        string.path(),
        "src/check.js",
        r#"const label = `${"first(); second(); \
third();"}`;
"#,
    )?;
    assert!(validate(string.path())?.status.success());
    Ok(())
}

#[test]
fn touched_loc_preserves_ordinary_double_quoted_delimiter_backslashes() -> TestResult {
    let body = fixture("scripts/check.sh", "exit 0\n".to_owned())?;
    write(
        body.path(),
        "scripts/check.sh",
        r#"cat <<"E\qOF"
first && second && third
E\qOF
"#,
    )?;
    assert!(validate(body.path())?.status.success());

    let after = fixture("scripts/check.sh", "exit 0\n".to_owned())?;
    write(
        after.path(),
        "scripts/check.sh",
        r#"cat <<"E\qOF"
payload
E\qOF
first && second && third
"#,
    )?;
    assert!(!validate(after.path())?.status.success());
    Ok(())
}
