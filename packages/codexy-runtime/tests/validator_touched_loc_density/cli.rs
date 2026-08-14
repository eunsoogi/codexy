use super::*;

#[test]
fn validator_cli_checks_density_in_a_changed_structured_file() -> TestResult {
    let repo = fixture("config/plugin.json", "{}\n".to_owned())?;
    write(
        repo.path(),
        "config/plugin.json",
        r#"{"one":1,"two":2,"three":3,"four":4}"#,
    )?;
    let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args(["--check-touched-loc", "--base-ref", "HEAD"])
        .current_dir(repo.path())
        .output()?;
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("dense JSON object"));
    Ok(())
}
