use super::*;

#[test]
fn touched_loc_handles_rust_module_visibility_forms() -> TestResult {
    for (declaration, allowed) in [
        ("mod extracted;", true),
        ("pub mod extracted;", true),
        ("pub(crate) mod extracted;", true),
        ("pub(super) module extracted;", false),
        ("pub(super) fn extracted();", false),
        ("pub(super) mod extracted;", true),
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

        assert_eq!(
            output.status.success(),
            allowed,
            "{declaration}\nstderr:\n{}",
            stderr(&output)
        );
    }
    Ok(())
}
