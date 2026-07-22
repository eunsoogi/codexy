use serde_json::json;
use std::{fs, path::Path, process::Command};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn invalid_issue_fails_without_mutating_existing_output() -> TestResult {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let temp = tempfile::tempdir()?;
    let issue = temp.path().join("issue.json");
    let taxonomy = temp.path().join("labels.json");
    let changes = temp.path().join("changes.txt");
    let output_dir = temp.path().join("metadata");
    fs::create_dir(&output_dir)?;
    fs::write(output_dir.join("sentinel"), b"unchanged\n")?;
    fs::write(
        &issue,
        serde_json::to_vec(&json!({
            "number": 301, "state": "CLOSED",
            "url": "https://github.com/eunsoogi/codexy/issues/301",
            "labels": [{"name": "type/ci"}, {"name": "area/release"}, {"name": "priority/medium"}],
            "milestone": {"title": "1.3.1"}, "assignees": [{"login": "eunsoogi"}]
        }))?,
    )?;
    fs::write(
        &taxonomy,
        serde_json::to_vec(&json!([
            {"name": "type/ci"}, {"name": "area/release"},
            {"name": "priority/medium"}, {"name": "status/review"}
        ]))?,
    )?;
    fs::write(&changes, "Cargo.toml\n")?;
    let result = Command::new(root.join("scripts/render-version-pr-metadata"))
        .args([
            "--version", "1.3.1", "--issue-json", text(&issue)?,
            "--repository-labels-json", text(&taxonomy)?,
            "--changed-files-file", text(&changes)?, "--output-dir", text(&output_dir)?,
        ])
        .output()?;
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("must be open"));
    assert_eq!(fs::read(output_dir.join("sentinel"))?, b"unchanged\n");
    assert!(!output_dir.join("title.txt").exists());
    assert!(!output_dir.join("body.md").exists());
    assert!(!output_dir.join("labels.json").exists());
    Ok(())
}

#[test]
fn pull_request_url_cannot_impersonate_governing_issue_without_mutation() -> TestResult {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let repository_name_too_long = format!(
        "https://github.com/eunsoogi/{}/issues/301",
        "a".repeat(101)
    );
    for url in [
        "https://github.com/eunsoogi/codexy/pull/301",
        "https://github.com/eunsoogi/codexy?redirect=/issues/301",
        "https://github.com/eunsoogi/codexy#/issues/301",
        "https://github.com//codexy/issues/301",
        "https://github.com/eun soo/codexy/issues/301",
        "https://github.com/eunsoogi/code xy/issues/301",
        "https://github.com/./codexy/issues/301",
        "https://github.com/eunsoogi/./issues/301",
        "https://github.com/eunsoogi-/codexy/issues/301",
        "https://github.com/eun--soogi/codexy/issues/301",
        repository_name_too_long.as_str(),
        "https://github.com/eunsoogi/codexy/issues/301/",
    ] {
        let temp = tempfile::tempdir()?;
        let issue = temp.path().join("issue.json");
        let taxonomy = temp.path().join("labels.json");
        let changes = temp.path().join("changes.txt");
        let output_dir = temp.path().join("metadata");
        fs::create_dir(&output_dir)?;
        fs::write(output_dir.join("sentinel"), b"unchanged\n")?;
        fs::write(
            &issue,
            serde_json::to_vec(&json!({
                "number": 301, "state": "OPEN", "url": url,
                "labels": [{"name": "type/ci"}, {"name": "area/release"}, {"name": "priority/medium"}],
                "milestone": {"title": "1.3.1"}, "assignees": [{"login": "eunsoogi"}]
            }))?,
        )?;
        fs::write(
            &taxonomy,
            serde_json::to_vec(&json!([
                {"name": "type/ci"}, {"name": "area/release"},
                {"name": "priority/medium"}, {"name": "status/review"}
            ]))?,
        )?;
        fs::write(&changes, "Cargo.toml\n")?;
        let result = Command::new(root.join("scripts/render-version-pr-metadata"))
            .args([
                "--version", "1.3.1", "--issue-json", text(&issue)?,
                "--repository-labels-json", text(&taxonomy)?,
                "--changed-files-file", text(&changes)?, "--output-dir", text(&output_dir)?,
            ])
            .output()?;
        assert!(!result.status.success(), "{url}");
        assert!(String::from_utf8_lossy(&result.stderr).contains("canonical issue URL"));
        assert_eq!(fs::read(output_dir.join("sentinel"))?, b"unchanged\n", "{url}");
        assert!(!output_dir.join("title.txt").exists());
        assert!(!output_dir.join("body.md").exists());
        assert!(!output_dir.join("labels.json").exists());
    }
    Ok(())
}

fn text(path: &Path) -> Result<&str, &'static str> {
    path.to_str().ok_or("non-UTF-8 test path")
}
