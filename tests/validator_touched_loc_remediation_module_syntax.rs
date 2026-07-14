mod support;

use support::touched_loc::{fixture, regular_lines, regular_lines_from, stderr, validate, write};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn touched_loc_does_not_credit_module_in_brace_macro_input() -> TestResult {
    assert_macro_input_not_credited("some_macro! {\n    mod helper;\n}\n")
}

#[test]
fn touched_loc_does_not_credit_module_in_paren_macro_input() -> TestResult {
    assert_macro_input_not_credited("some_macro!(\n    mod helper;\n);\n")
}

#[test]
fn touched_loc_does_not_credit_module_in_bracket_macro_input() -> TestResult {
    assert_macro_input_not_credited("some_macro![\n    mod helper;\n];\n")
}

#[test]
fn touched_loc_credits_outer_module_after_macro_inputs() -> TestResult {
    for macro_invocation in [
        "some_macro! {\n    mod ignored;\n}\n",
        "some_macro!(\n    mod ignored;\n);\n",
        "some_macro![\n    mod ignored;\n];\n",
    ] {
        let repo = module_fixture(
            "src/foo/helper.rs",
            &format!("{macro_invocation}mod helper;\n"),
        )?;
        let output = validate(repo.path())?;
        assert!(
            output.status.success(),
            "macro invocation: {macro_invocation}\nstderr:\n{}",
            stderr(&output)
        );
    }
    Ok(())
}

#[test]
fn touched_loc_resolves_raw_identifier_file_module() -> TestResult {
    let repo = module_fixture("src/foo/async.rs", "mod r#async;\n")?;
    let output = validate(repo.path())?;
    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    Ok(())
}

#[test]
fn touched_loc_resolves_raw_identifier_directory_module() -> TestResult {
    let repo = module_fixture("src/foo/async/mod.rs", "mod r#async;\n")?;
    let output = validate(repo.path())?;
    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    Ok(())
}

#[test]
fn touched_loc_does_not_credit_literal_raw_identifier_path() -> TestResult {
    let repo = module_fixture("src/foo/r#async.rs", "mod r#async;\n")?;
    let output = validate(repo.path())?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("multiline collapse"));
    Ok(())
}

#[test]
fn touched_loc_still_resolves_ordinary_module_identifier() -> TestResult {
    let repo = module_fixture("src/foo/helper.rs", "mod helper;\n")?;
    let output = validate(repo.path())?;
    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    Ok(())
}

fn assert_macro_input_not_credited(declaration: &str) -> TestResult {
    let repo = module_fixture("src/foo/helper.rs", declaration)?;
    let output = validate(repo.path())?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("multiline collapse"));
    Ok(())
}

fn module_fixture(extracted_path: &str, declaration: &str) -> TestResult<tempfile::TempDir> {
    let repo = fixture("src/foo.rs", regular_lines(252))?;
    let retained_lines = 250 - declaration.lines().count();
    write(
        repo.path(),
        "src/foo.rs",
        &format!("{declaration}{}", regular_lines(retained_lines)),
    )?;
    write(
        repo.path(),
        extracted_path,
        &regular_lines_from(retained_lines, 252 - retained_lines),
    )?;
    Ok(repo)
}
