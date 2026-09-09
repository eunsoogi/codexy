use super::*;

#[test]
fn touched_loc_rejects_visibility_on_non_module_items() -> TestResult {
    for declaration in [
        "pub(super) module extracted;",
        "pub(super) fn extracted();",
    ] {
        let repo = fixture("src/too_large.rs", multiline_source())?;
        write(
            repo.path(),
            "src/too_large.rs",
            &format!("{declaration}\n{}", regular_lines(249)),
        )?;
        write(
            repo.path(),
            "src/too_large/extracted.rs",
            "let summary = format!(\n    \"status\"\n);\n",
        )?;

        let output = validate(repo.path())?;

        assert!(
            !output.status.success(),
            "{declaration}\nstderr:\n{}",
            stderr(&output)
        );
    }
    Ok(())
}
