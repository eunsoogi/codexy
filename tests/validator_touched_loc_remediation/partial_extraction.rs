use super::*;

#[test]
fn touched_loc_rejects_half_extraction_with_remaining_multiline_collapse() -> TestResult {
    let repo = fixture("src/too_large.rs", multiline_source())?;
    write(
        repo.path(),
        "src/too_large.rs",
        &format!(
            "mod helper;\n{}let summary = format!(\"status\");\n",
            regular_lines(247)
        ),
    )?;
    write(repo.path(), "src/helper.rs", &regular_lines_from(247, 2))?;

    let output = validate(repo.path())?;

    assert!(!output.status.success(), "stderr:\n{}", stderr(&output));
    assert!(stderr(&output).contains("multiline collapse"));
    Ok(())
}

#[test]
fn touched_loc_allows_three_quarter_extraction_coverage() -> TestResult {
    let repo = fixture("src/too_large.rs", regular_lines(252))?;
    write(
        repo.path(),
        "src/too_large.rs",
        &format!("mod helper;\n{}", regular_lines(248)),
    )?;
    write(repo.path(), "src/helper.rs", &regular_lines_from(248, 3))?;

    let output = validate(repo.path())?;

    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    Ok(())
}
