use std::path::Path;
use std::process::{Command, Output};

mod support;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const REQUIRED_MARKERS: &[&str] = &[
    "MUST identify formatting-only LOC remediation before approving readiness.",
    "MUST inspect the base-to-current reduction and block blank-line deletion or collapsed readable multiline code, tests, or instructions",
    "MUST permit a collapsed readable multiline construct when the same reduction includes independent structural remediation.",
];

#[test]
fn roles_validator_requires_sentinel_loc_remediation_evidence() -> TestResult {
    for marker in REQUIRED_MARKERS {
        let (_temp, plugin_root) = fixture()?;
        let sentinel_path = plugin_root.join("agents/codexy-sentinel.toml");
        let sentinel = std::fs::read_to_string(&sentinel_path)?;
        assert!(
            sentinel.contains(marker),
            "missing fixture marker: {marker}"
        );
        std::fs::write(&sentinel_path, sentinel.replace(marker, "MUST review LOC."))?;

        let output = validator(&plugin_root)?;

        assert!(!output.status.success());
        assert!(stderr(&output).contains("reviewer gate contract is missing"));
    }
    Ok(())
}

#[test]
fn roles_validator_rejects_quoted_or_negated_loc_remediation_markers() -> TestResult {
    for wrapper in [
        "MUST NOT {marker}",
        "Quoted policy text: \"{marker}\"",
        "Quoted policy text: “{marker}”",
        "Quoted policy text: “The policy says {marker}”",
        "Quoted policy text: ‘The policy says {marker}’",
        "Quoted policy text: ” stray closer, then “The policy says {marker}”",
        "Quoted policy text: 'The policy says {marker}'",
        "Quoted policy text said 'reviewers' reported {marker}",
        "Quoted policy text: `{marker} is required`",
    ] {
        let (_temp, plugin_root) = fixture()?;
        let sentinel_path = plugin_root.join("agents/codexy-sentinel.toml");
        let sentinel = std::fs::read_to_string(&sentinel_path)?;
        let marker = REQUIRED_MARKERS[0];
        std::fs::write(
            &sentinel_path,
            sentinel.replace(marker, &wrapper.replace("{marker}", marker)),
        )?;

        let output = validator(&plugin_root)?;

        assert!(
            !output.status.success(),
            "wrapper unexpectedly passed: {wrapper}"
        );
        assert!(stderr(&output).contains("reviewer gate contract is missing"));
    }
    Ok(())
}

#[test]
fn roles_validator_accepts_closed_typographic_quote_before_marker() -> TestResult {
    let (_temp, plugin_root) = fixture()?;
    let sentinel_path = plugin_root.join("agents/codexy-sentinel.toml");
    let sentinel = std::fs::read_to_string(&sentinel_path)?;
    let marker = REQUIRED_MARKERS[0];
    std::fs::write(
        &sentinel_path,
        sentinel.replacen(marker, &format!("“historical wording” {marker}"), 1),
    )?;

    let output = validator(&plugin_root)?;
    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    Ok(())
}

#[test]
fn roles_validator_accepts_closed_straight_quote_before_marker() -> TestResult {
    for quoted in [
        "'historical wording'",
        "'historical policies' before",
        "`historical wording`",
    ] {
        let (_temp, plugin_root) = fixture()?;
        let sentinel_path = plugin_root.join("agents/codexy-sentinel.toml");
        let sentinel = std::fs::read_to_string(&sentinel_path)?;
        let marker = REQUIRED_MARKERS[0];
        std::fs::write(
            &sentinel_path,
            sentinel.replacen(marker, &format!("{quoted} {marker}"), 1),
        )?;
        let output = validator(&plugin_root)?;
        assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    }
    Ok(())
}

fn fixture() -> TestResult<(tempfile::TempDir, std::path::PathBuf)> {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    support::copy_dir(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy"),
        &plugin_root,
    )?;
    Ok((temp, plugin_root))
}

fn validator(plugin_root: &Path) -> TestResult<Output> {
    Ok(Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--plugin-root",
            plugin_root.to_str().ok_or("plugin root")?,
            "--check-roles",
        ])
        .output()?)
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
