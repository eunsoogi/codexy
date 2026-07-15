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
fn touched_loc_rejects_unrelated_extracted_boilerplate() -> TestResult {
    let repo = fixture("src/too_large.rs", regular_lines(252))?;
    write(
        repo.path(),
        "src/too_large.rs",
        &format!("mod helper;\n{}", regular_lines(249)),
    )?;
    write(repo.path(), "src/helper.rs", &unrelated_lines(3))?;

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

#[test]
fn touched_loc_allows_large_transformed_extraction_with_exact_line_coverage() -> TestResult {
    let repo = fixture("src/too_large.rs", regular_lines(252))?;
    write(
        repo.path(),
        "src/too_large.rs",
        &format!("mod helper;\n{}", regular_lines(52)),
    )?;
    write(
        repo.path(),
        "src/helper.rs",
        &(regular_lines_from(52, 108) + &unrelated_lines(92)),
    )?;

    let output = validate(repo.path())?;

    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    Ok(())
}

fn unrelated_lines(count: usize) -> String {
    (0..count)
        .map(|index| format!("fn unrelated_{index}() {{}}\n"))
        .collect()
}
