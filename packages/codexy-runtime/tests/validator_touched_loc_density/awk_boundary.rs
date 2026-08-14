use super::*;

#[test]
fn touched_loc_ignores_an_embedded_awk_parser() -> TestResult {
    let repo = fixture("scripts/parser.sh", "exit 0\n".to_owned())?;
    write(
        repo.path(),
        "scripts/parser.sh",
        "awk '\nfunction value() { first(); second(); third(); }\n'\n",
    )?;
    assert!(validate(repo.path())?.status.success());
    Ok(())
}
