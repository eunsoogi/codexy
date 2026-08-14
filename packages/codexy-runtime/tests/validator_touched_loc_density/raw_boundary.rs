use super::*;

#[test]
fn touched_loc_ignores_embedded_raw_string_fixtures() -> TestResult {
    let repo = fixture("src/lib.rs", "pub fn readable() {}\n".to_owned())?;
    write(
        repo.path(),
        "src/lib.rs",
        "let probe = r#\"\nfirst(); second(); third();\n\"#;\n",
    )?;
    assert!(validate(repo.path())?.status.success());

    let inline = fixture("src/lib.rs", "pub fn readable() {}\n".to_owned())?;
    write(
        inline.path(),
        "src/lib.rs",
        "fn compact() { let fixture = r#\"first(); second(); third();\"#; first(); second(); third(); }\n",
    )?;
    assert!(!validate(inline.path())?.status.success());

    let closing = fixture("src/lib.rs", "pub fn readable() {}\n".to_owned())?;
    write(
        closing.path(),
        "src/lib.rs",
        "fn compact() { let fixture = r#########\"\nfirst(); second(); third();\n\"#########; if ready() { first(); second(); third(); } }\n",
    )?;
    assert!(!validate(closing.path())?.status.success());

    let comment = fixture("src/lib.rs", "pub fn readable() {}\n".to_owned())?;
    write(
        comment.path(),
        "src/lib.rs",
        "/* r\"\nfixture(); fixture(); fixture();\n*/\nfn compact() { first(); second(); third(); }\n",
    )?;
    assert!(!validate(comment.path())?.status.success());
    Ok(())
}
