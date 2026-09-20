use std::path::{Path, PathBuf};
use std::process::Stdio;

use serde_json::{Value, json};

use super::super::*;
use crate::support::FixtureCommand;

pub(super) struct GitFixture {
    _temp: tempfile::TempDir,
    pub(super) root: PathBuf,
    pub(super) base: String,
    pub(super) head: String,
}

pub(super) fn run_demo(
    base_files: &[(&str, &str)],
    changed_files: &[(&str, &str)],
    mapping: Value,
    check_id: &str,
    expected_path: &str,
    dependency_state: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = git_fixture(base_files, changed_files)?;
    let (_installed, mut client) = installed_codegraph_client()?;
    let list = client.send(&json!({
        "jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {}
    }))?;
    let tools = list["result"]["tools"].as_array().ok_or("tools list")?;
    for name in [
        "codegraph_index",
        "codegraph_change_impact",
        "codegraph_check_selection",
    ] {
        assert!(tools.iter().any(|tool| tool["name"] == name), "missing {name}");
    }

    let root = fixture.root.to_string_lossy();
    let impact_args = json!({
        "root": root,
        "base": fixture.base.clone(),
        "head": fixture.head.clone(),
        "maxFiles": 80,
        "maxPaths": 256
    });
    let impact = tool_payload(&mut client, 3, "codegraph_change_impact", impact_args)?;
    assert_eq!(impact["changeSet"]["observed_state"]["mode"], "head_comparison");
    assert!(
        impact["impact"]["limits"].is_object(),
        "impact limits must be visible"
    );
    assert!(
        impact["explanation"].as_str().is_some(),
        "impact explanation must be readable"
    );

    let selection_args = json!({
        "root": root,
        "base": fixture.base,
        "head": fixture.head,
        "mappings": {
            "checks": [{
                "id": check_id,
                "command": format!(
                    "touch {}",
                    fixture.root.join("recommendation-was-executed").display()
                ),
                "description": format!("check for {check_id}")
            }],
            "mappings": [mapping]
        },
        "dependencyState": dependency_state
    });
    let selection = tool_payload(&mut client, 4, "codegraph_check_selection", selection_args)?;
    let recommendations = selection["selection"]["recommendations"]
        .as_array()
        .ok_or("recommendations")?;
    let recommendation = recommendations
        .iter()
        .find(|recommendation| recommendation["id"] == check_id)
        .ok_or("expected recommendation")?;
    assert!(recommendation["paths"]
        .as_array()
        .is_some_and(|paths| paths.iter().any(|path| path == expected_path)));
    assert!(
        recommendation["reasons"]
            .as_array()
            .is_some_and(|reasons| !reasons.is_empty()),
        "recommendation reasons must be visible"
    );
    assert!(selection["selection"]["impact_unknown"].is_array());
    assert!(selection["selection"]["broader_verification"].is_array());
    assert_eq!(
        selection["proofBoundary"],
        json!({
            "checksExecuted": false,
            "checksWaived": false,
            "completionDecided": false,
            "hostExposure": "unobserved"
        })
    );
    assert!(
        selection["explanation"].as_str().is_some(),
        "selection explanation must be readable"
    );
    assert!(!fixture.root.join("recommendation-was-executed").exists());
    Ok(())
}

pub(super) fn git_fixture(
    base_files: &[(&str, &str)],
    changed_files: &[(&str, &str)],
) -> Result<GitFixture, Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let root = temp.path().join("repository");
    std::fs::create_dir_all(&root)?;
    git(&root, &["init", "-q"])?;
    git(&root, &["config", "user.email", "codexy-tests@example.invalid"])?;
    git(&root, &["config", "user.name", "Codexy Tests"])?;
    write_files(&root, base_files)?;
    git(&root, &["add", "--all"])?;
    git(&root, &["commit", "-qm", "base"])?;
    let base = git(&root, &["rev-parse", "HEAD"])?;
    write_files(&root, changed_files)?;
    git(&root, &["add", "--all"])?;
    git(&root, &["commit", "-qm", "change"])?;
    let head = git(&root, &["rev-parse", "HEAD"])?;
    Ok(GitFixture {
        _temp: temp,
        root,
        base,
        head,
    })
}

pub(super) fn installed_codegraph_client(
) -> Result<(InstalledPlugin, McpClient), Box<dyn std::error::Error>> {
    let installed = installed_plugin_copy()?;
    let mut command = FixtureCommand::new(installed.path.join("mcp/codexy-mcp-codegraph"));
    installed.use_candidate_runtime(&mut command);
    command
        .current_dir(&installed.path)
        .env("PATH", "/usr/bin:/bin")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut client = McpClient::spawn_command(command)?;
    let init = client.send(&json!({
        "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}
    }))?;
    assert_eq!(init["result"]["serverInfo"]["name"], "codexy-codegraph");
    Ok((installed, client))
}

pub(super) fn tool_payload(
    client: &mut McpClient,
    id: u64,
    name: &str,
    arguments: Value,
) -> Result<Value, Box<dyn std::error::Error>> {
    let response = client.send(&json!({
        "jsonrpc": "2.0", "id": id, "method": "tools/call",
        "params": {"name": name, "arguments": arguments}
    }))?;
    if response.get("error").is_some() {
        return Err(format!("{name} failed: {response}").into());
    }
    let text = response["result"]["content"][0]["text"]
        .as_str()
        .ok_or("missing tool payload text")?;
    Ok(serde_json::from_str(text)?)
}

fn write_files(root: &Path, files: &[(&str, &str)]) -> Result<(), Box<dyn std::error::Error>> {
    for (relative, contents) in files {
        let path = root.join(relative);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, contents)?;
    }
    Ok(())
}

fn git(root: &Path, args: &[&str]) -> Result<String, Box<dyn std::error::Error>> {
    let output = std::process::Command::new("git")
        .args(args)
        .current_dir(root)
        .output()?;
    if !output.status.success() {
        return Err(format!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}
