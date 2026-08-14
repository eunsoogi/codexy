use std::process::Command;

use crate::support;
use support::touched_loc::{fixture, stderr, validate, write};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn rejects_representative_compacted_maintained_source() -> TestResult {
    let cases = [
        (
            "src/lib.rs",
            "pub fn compact() { first(); second(); third(); }\n",
            "dense Rust",
        ),
        ("scripts/release.sh", "first && second && third\n", "dense command"),
        (
            "config/plugin.json",
            r#"{"one":1,"two":2,"three":3,"four":4}"#,
            "dense JSON",
        ),
        (
            "plugins/codexy/skills/example/SKILL.md",
            "Identify the owner; retain the evidence; avoid duplicate work.\n",
            "dense Markdown",
        ),
    ];
    for (path, text, expected) in cases {
        let repo = fixture(path, "readable\n".to_owned())?;
        write(repo.path(), path, text)?;
        let output = validate(repo.path())?;
        assert!(!output.status.success(), "{}", stderr(&output));
        assert!(stderr(&output).contains(expected));
    }
    Ok(())
}

#[test]
fn preserves_quoted_values_comments_and_small_constructs() -> TestResult {
    let cases = [
        ("src/lib.rs", "// first(); second(); third();\n"),
        (
            "src/lib.rs",
            "const URL: &str = \"https://example.test/a;b;c\";\n",
        ),
        ("scripts/release.sh", "first && second\n"),
        ("config/plugin.json", r#"{"one":1,"two":2,"three":3}"#),
        ("plugins/codexy/skills/example/SKILL.md", "MUST retain evidence.\n"),
    ];
    for (path, text) in cases {
        let repo = fixture(path, "readable\n".to_owned())?;
        write(repo.path(), path, text)?;
        let output = validate(repo.path())?;
        assert!(output.status.success(), "{}", stderr(&output));
    }
    Ok(())
}

#[test]
fn provenance_requires_declared_source_context() -> TestResult {
    let fixture_repo = fixture(
        "packages/getcodexy/tests/fixtures/component-installation-cases.json",
        "{}\n".to_owned(),
    )?;
    write(
        fixture_repo.path(),
        "packages/getcodexy/tests/fixtures/component-installation-cases.json",
        r#"{"schema":"getcodexy.component-installation-cases.v1","one":1,"two":2,"three":3,"four":4}"#,
    )?;
    assert!(validate(fixture_repo.path())?.status.success());

    let maintained_repo = fixture("tests/fixtures/decoy.json", "{}\n".to_owned())?;
    write(
        maintained_repo.path(),
        "tests/fixtures/decoy.json",
        r#"{"one":1,"two":2,"three":3,"four":4}"#,
    )?;
    assert!(!validate(maintained_repo.path())?.status.success());

    let generated_repo = fixture("target/debug/output.json", "{}\n".to_owned())?;
    write(
        generated_repo.path(),
        "target/debug/output.json",
        r#"{"one":1,"two":2,"three":3,"four":4}"#,
    )?;
    assert!(validate(generated_repo.path())?.status.success());
    Ok(())
}

#[test]
fn inventory_is_source_addressable_and_classified() -> TestResult {
    let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .arg("--print-density-inventory")
        .current_dir(codexy_runtime::paths::repository_root())
        .output()?;
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let report = String::from_utf8(output.stdout)?;
    assert!(report.lines().all(|line| line.contains("\taudit-input=structural-density")));
    assert!(report.lines().any(|line| line.contains("\texact-fixture\t")));
    assert!(report.lines().any(|line| line.contains("\tgenerated\t")));
    assert!(report.contains("maintained-readable/manual-audit"));
    assert!(!report.contains("confirmed-density-defect"));
    Ok(())
}
