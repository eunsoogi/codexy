use super::*;

#[test]
fn touched_loc_allows_extraction_through_a_custom_rust_module_path() -> TestResult {
    let repo = fixture("src/too_large.rs", regular_lines(252))?;
    write(
        repo.path(),
        "src/too_large.rs",
        &format!(
            "#[path = \"custom.rs\"]\nmod extracted;\n{}",
            regular_lines(248)
        ),
    )?;
    write(repo.path(), "src/custom.rs", &regular_lines_from(248, 4))?;

    let output = validate(repo.path())?;

    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    Ok(())
}

#[test]
fn touched_loc_rejects_invalid_custom_rust_module_paths() -> TestResult {
    for attribute in ["#[path = custom.rs]", r#"#[path = \"custom.rs\"]"#] {
        let repo = fixture("src/too_large.rs", regular_lines(252))?;
        write(
            repo.path(),
            "src/too_large.rs",
            &format!("{attribute}\nmod extracted;\n{}", regular_lines(248)),
        )?;
        write(repo.path(), "src/custom.rs", &regular_lines_from(248, 4))?;

        let output = validate(repo.path())?;

        assert!(
            !output.status.success(),
            "invalid custom module attribute must be rejected: {attribute}\nstderr:\n{}",
            stderr(&output)
        );
    }
    Ok(())
}

#[test]
fn touched_loc_keeps_default_rust_module_paths_eligible() -> TestResult {
    let repo = fixture("src/too_large.rs", regular_lines(252))?;
    write(
        repo.path(),
        "src/too_large.rs",
        &format!("mod extracted;\n{}", regular_lines(249)),
    )?;
    write(repo.path(), "src/extracted.rs", &regular_lines_from(249, 3))?;

    let output = validate(repo.path())?;

    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    Ok(())
}
