use super::GateFixture;

#[test]
fn gate_runs_the_exact_full_workload_once() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = GateFixture::new(0, 1802, 0)?;
    let output = fixture.run(&[])?;

    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        std::fs::read_to_string(&fixture.marker)?,
        "test --locked --all-targets\n"
    );
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.contains("integration-targets\t2\tPASS"), "{stdout}");
    assert!(
        stdout.contains("tests\t1802 passed\t0 failed\t0 ignored\tPASS"),
        "{stdout}"
    );
    assert!(
        stdout.contains("archive-fixture-nested-cargo-builds\t0\tPASS"),
        "{stdout}"
    );
    assert!(stdout.contains("compile-seconds\t62.000"), "{stdout}");
    assert!(stdout.contains("budget-seconds\t195.000"), "{stdout}");
    assert!(stdout.ends_with("result\tPASS\n"), "{stdout}");
    Ok(())
}
