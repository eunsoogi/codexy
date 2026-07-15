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
    assert_rustc_accepts(repo.path())?;

    let output = validate(repo.path())?;

    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    Ok(())
}

#[test]
fn touched_loc_rejects_mechanical_custom_rust_module_paths() -> TestResult {
    let repo = fixture("src/too_large.rs", regular_lines(252))?;
    write(
        repo.path(),
        "src/too_large.rs",
        &format!(
            "#[path = \"part-1.rs\"]\nmod extracted;\n{}",
            regular_lines(248)
        ),
    )?;
    write(repo.path(), "src/part-1.rs", &regular_lines_from(248, 4))?;
    assert_rustc_accepts(repo.path())?;

    let output = validate(repo.path())?;

    assert!(!output.status.success(), "stderr:\n{}", stderr(&output));
    assert!(stderr(&output).contains("multiline collapse"));
    Ok(())
}

#[test]
fn touched_loc_allows_compact_custom_rust_module_path_syntax() -> TestResult {
    let repo = fixture("src/too_large.rs", regular_lines(252))?;
    write(
        repo.path(),
        "src/too_large.rs",
        &format!(
            "#[path=\"custom.rs\"]\nmod extracted;\n{}",
            regular_lines(248)
        ),
    )?;
    write(repo.path(), "src/custom.rs", &regular_lines_from(248, 4))?;

    let output = validate(repo.path())?;

    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    Ok(())
}

#[test]
fn touched_loc_allows_same_line_custom_rust_module_path_syntax() -> TestResult {
    let repo = fixture("src/too_large.rs", regular_lines(252))?;
    write(
        repo.path(),
        "src/too_large.rs",
        &format!(
            "#[path = \"custom.rs\"] mod extracted;\n{}",
            regular_lines(249)
        ),
    )?;
    write(repo.path(), "src/custom.rs", &regular_lines_from(249, 3))?;

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
fn touched_loc_rejects_custom_path_detached_from_module_declaration() -> TestResult {
    let repo = fixture("src/too_large.rs", regular_lines(252))?;
    write(
        repo.path(),
        "src/too_large.rs",
        &format!(
            "#[path=\"custom.rs\"]\nfn unrelated() {{}}\nmod extracted;\n{}",
            regular_lines(247)
        ),
    )?;
    write(repo.path(), "src/custom.rs", &regular_lines_from(247, 5))?;

    let output = validate(repo.path())?;

    assert!(!output.status.success(), "stderr:\n{}", stderr(&output));
    assert!(stderr(&output).contains("multiline collapse"));
    Ok(())
}

#[test]
fn touched_loc_rejects_same_line_path_without_module_declaration() -> TestResult {
    let repo = fixture("src/too_large.rs", regular_lines(252))?;
    write(
        repo.path(),
        "src/too_large.rs",
        &format!(
            "#[path = \"custom.rs\"] fn unrelated() {{}}\n{}",
            regular_lines(249)
        ),
    )?;
    write(repo.path(), "src/custom.rs", &regular_lines_from(249, 3))?;

    let output = validate(repo.path())?;

    assert!(!output.status.success(), "stderr:\n{}", stderr(&output));
    assert!(stderr(&output).contains("multiline collapse"));
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
    write(
        repo.path(),
        "src/too_large/extracted.rs",
        &regular_lines_from(249, 3),
    )?;

    let output = validate(repo.path())?;

    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    Ok(())
}

fn assert_rustc_accepts(root: &std::path::Path) -> TestResult {
    let output = std::process::Command::new("rustc")
        .args(["--crate-type=lib", "src/too_large.rs", "--out-dir", "."])
        .current_dir(root)
        .output()?;
    assert!(output.status.success(), "rustc: {}", stderr(&output));
    Ok(())
}
