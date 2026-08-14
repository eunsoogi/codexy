use std::process::Command;

use crate::support;
use support::touched_loc::{fixture, stderr, validate, write};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn touched_loc_rejects_compacted_maintained_source() -> TestResult {
    let repo = fixture("src/lib.rs", "pub fn readable() {}\n".to_owned())?;
    write(
        repo.path(),
        "src/lib.rs",
        "pub fn compact() { first(); second(); third(); }\n",
    )?;
    let output = validate(repo.path())?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("dense Rust statements"));
    Ok(())
}

#[test]
fn touched_loc_checks_maintained_fixture_and_structured_source() -> TestResult {
    let fixture_repo = fixture("tests/fixtures/maintained.py", "pass\n".to_owned())?;
    write(
        fixture_repo.path(),
        "tests/fixtures/maintained.py",
        "first(); second(); third()\n",
    )?;
    let fixture_output = validate(fixture_repo.path())?;
    assert!(!fixture_output.status.success(), "{}", stderr(&fixture_output));

    let json_repo = fixture("config/plugin.json", "{}\n".to_owned())?;
    write(
        json_repo.path(),
        "config/plugin.json",
        r#"{"one":1,"two":2,"three":3,"four":4}"#,
    )?;
    let json_output = validate(json_repo.path())?;
    assert!(!json_output.status.success(), "{}", stderr(&json_output));
    assert!(stderr(&json_output).contains("dense JSON object"));
    Ok(())
}

#[test]
fn touched_loc_preserves_source_backed_exact_fixtures_and_long_readable_line() -> TestResult {
    let fixture_repo = fixture("tests/fixtures/malformed_input.py", "pass\n".to_owned())?;
    write(
        fixture_repo.path(),
        "tests/fixtures/malformed_input.py",
        "# malformed input remains maintained\nfirst(); second(); third()\n",
    )?;
    let fixture_output = validate(fixture_repo.path())?;
    assert!(!fixture_output.status.success(), "{}", stderr(&fixture_output));

    let json_fixture = fixture(
        "packages/getcodexy/tests/fixtures/component-installation-cases.json",
        "{}\n".to_owned(),
    )?;
    write(
        json_fixture.path(),
        "packages/getcodexy/tests/fixtures/component-installation-cases.json",
        r#"{"schema":"getcodexy.component-installation-cases.v1","fixtures":[]}"#,
    )?;
    assert!(validate(json_fixture.path())?.status.success());

    let json_decoy = fixture("tests/fixtures/maintained.json", "{}\n".to_owned())?;
    write(
        json_decoy.path(),
        "tests/fixtures/maintained.json",
        r#"{"one":1,"two":2,"three":3,"four":4}"#,
    )?;
    assert!(!validate(json_decoy.path())?.status.success());

    let line = format!("const URL: &str = \"https://example.test/{}\";\n", "x".repeat(220));
    let readable_repo = fixture("src/lib.rs", "pub fn readable() {}\n".to_owned())?;
    write(readable_repo.path(), "src/lib.rs", &line)?;
    let readable_output = validate(readable_repo.path())?;
    assert!(readable_output.status.success(), "{}", stderr(&readable_output));
    Ok(())
}

#[test]
fn touched_loc_distinguishes_maintained_tests_from_exact_fixtures() -> TestResult {
    let repo = fixture("tests/mcp_response_checker.rs", "fn readable() {}\n".to_owned())?;
    write(
        repo.path(),
        "tests/mcp_response_checker.rs",
        "fn compact() { first(); second(); third(); }\n",
    )?;
    let output = validate(repo.path())?;
    assert!(!output.status.success(), "{}", stderr(&output));
    assert!(stderr(&output).contains("dense Rust statements"));
    Ok(())
}

#[test]
fn touched_loc_detects_dense_markdown_but_preserves_instruction_boundaries() -> TestResult {
    let dense = fixture("plugins/codexy/skills/example/SKILL.md", "Readable text.\n".to_owned())?;
    write(
        dense.path(),
        "plugins/codexy/skills/example/SKILL.md",
        "Identify the owner; retain the evidence; avoid duplicate work.\n",
    )?;
    assert!(!validate(dense.path())?.status.success());

    let boundary = fixture("plugins/codexy/skills/example/SKILL.md", "Readable text.\n".to_owned())?;
    write(
        boundary.path(),
        "plugins/codexy/skills/example/SKILL.md",
        "A rule MUST identify the owner and MUST retain evidence.\n",
    )?;
    assert!(validate(boundary.path())?.status.success());

    let prose = fixture("plugins/codexy/skills/example/SKILL.md", "Readable text.\n".to_owned())?;
    write(
        prose.path(),
        "plugins/codexy/skills/example/SKILL.md",
        "Identify the owner; retain the evidence; avoid duplicate work.\n",
    )?;
    assert!(!validate(prose.path())?.status.success());
    Ok(())
}

#[test]
fn touched_loc_ignores_comments_and_handles_urls_and_escaped_quotes() -> TestResult {
    let comment = fixture("src/lib.rs", "pub fn readable() {}\n".to_owned())?;
    write(
        comment.path(),
        "src/lib.rs",
        "// fn compact() { first(); second(); third(); }\n",
    )?;
    assert!(validate(comment.path())?.status.success());

    let json = fixture("config/plugin.json", "{}\n".to_owned())?;
    write(
        json.path(),
        "config/plugin.json",
        r#"{"url":"https://example.test/\"//","one":1,"two":2,"three":3,"four":4}"#,
    )?;
    let output = validate(json.path())?;
    assert!(!output.status.success(), "{}", stderr(&output));
    assert!(stderr(&output).contains("dense JSON object"));
    Ok(())
}

#[path = "validator_touched_loc_density/raw_boundary.rs"]
mod raw_boundary;

#[path = "validator_touched_loc_density/awk_boundary.rs"]
mod awk_boundary;
#[path = "validator_touched_loc_density/markdown_boundary.rs"]
mod markdown_boundary;
#[path = "validator_touched_loc_density/lexer_boundary.rs"]
mod lexer_boundary;
#[path = "validator_touched_loc_density/sentinel_v4_boundary.rs"]
mod sentinel_v4_boundary;
#[path = "validator_touched_loc_density/cli.rs"]
mod cli;

#[test]
fn touched_loc_handles_language_specific_nesting_and_boundaries() -> TestResult {
    let rust_repo = fixture("src/lib.rs", "pub fn readable() {}\n".to_owned())?;
    write(
        rust_repo.path(),
        "src/lib.rs",
        "pub fn outer() { if ready() { first(); second(); third(); } }\n",
    )?;
    assert!(!validate(rust_repo.path())?.status.success());

    let shell_repo = fixture("scripts/release.sh", "echo ready\n".to_owned())?;
    write(shell_repo.path(), "scripts/release.sh", "first && second && third\n")?;
    assert!(!validate(shell_repo.path())?.status.success());

    let shell_boundary = fixture("scripts/release.sh", "echo ready\n".to_owned())?;
    write(shell_boundary.path(), "scripts/release.sh", "first && second\n")?;
    assert!(validate(shell_boundary.path())?.status.success());

    let shell_case = fixture("scripts/release.sh", "echo ready\n".to_owned())?;
    write(shell_case.path(), "scripts/release.sh", "case \"$name\" in value) ;; esac\n")?;
    assert!(validate(shell_case.path())?.status.success());

    let shell_conditional = fixture("scripts/release.sh", "echo ready\n".to_owned())?;
    write(shell_conditional.path(), "scripts/release.sh", "if first || second; then third; fi\n")?;
    assert!(validate(shell_conditional.path())?.status.success());

    let toml_repo = fixture("config/settings.toml", "name = \"app\"\n".to_owned())?;
    write(
        toml_repo.path(),
        "config/settings.toml",
        "item = { one = 1, two = 2, three = 3, four = 4 }\n",
    )?;
    assert!(!validate(toml_repo.path())?.status.success());

    let yaml_repo = fixture(".github/workflows/check.yml", "name: check\n".to_owned())?;
    write(
        yaml_repo.path(),
        ".github/workflows/check.yml",
        "matrix: { one: 1, two: 2, three: 3, four: 4 }\n",
    )?;
    assert!(!validate(yaml_repo.path())?.status.success());

    let json_boundary = fixture("config/plugin.json", "{}\n".to_owned())?;
    write(json_boundary.path(), "config/plugin.json", r#"{"one":1,"two":2,"three":3}"#)?;
    assert!(validate(json_boundary.path())?.status.success());

    let lifetime = fixture("src/lib.rs", "pub fn readable() {}\n".to_owned())?;
    write(lifetime.path(), "src/lib.rs", "fn borrow<'a>(item: &'a str) -> &'a str { item }\n")?;
    assert!(validate(lifetime.path())?.status.success());

    let javascript = fixture("src/check.js", "const ready = true;\n".to_owned())?;
    write(javascript.path(), "src/check.js", "for (;;) { break; }\n")?;
    assert!(validate(javascript.path())?.status.success());

    write(
        javascript.path(),
        "src/check.js",
        "for (;;) { first(); second(); third(); }\n",
    )?;
    assert!(!validate(javascript.path())?.status.success());

    let rust_boundary = fixture("src/lib.rs", "pub fn readable() {}\n".to_owned())?;
    write(rust_boundary.path(), "src/lib.rs", "fn compact() { first(); second(); }\n")?;
    assert!(validate(rust_boundary.path())?.status.success());

    let python = fixture("scripts/check.py", "pass\n".to_owned())?;
    write(python.path(), "scripts/check.py", "if ready: first(); second(); third()\n")?;
    assert!(!validate(python.path())?.status.success());
    Ok(())
}

#[test]
fn validator_cli_emits_a_source_addressable_density_inventory() -> TestResult {
    let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .arg("--print-density-inventory")
        .current_dir(codexy_runtime::paths::repository_root())
        .output()?;
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let report = String::from_utf8(output.stdout)?;
    assert!(report.lines().all(|line| line.contains("\taudit-input=")));
    assert!(report.lines().any(|line| line.contains("audit-input=structural-density")));
    assert!(report.lines().all(|line| line.contains("structural-density")));
    Ok(())
}
