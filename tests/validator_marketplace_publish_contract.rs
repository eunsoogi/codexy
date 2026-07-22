use std::process::Command;

use serde_yaml::Value;

#[test]
fn runtime_check_workflow_assembles_state_aware_immutable_packages() -> Result<(), Box<dyn std::error::Error>> {
    let workflow = document("plugin-runtime-binaries.yml")?;
    let job = &workflow["jobs"]["verify-selected-package"];
    assert_eq!(job["strategy"]["matrix"]["include"].as_sequence().ok_or("matrix")?.len(), 2);
    let download = run(job, "Download and verify selected immutable bytes")?;
    assert!(lines(download).any(|line| line == "curl --fail --location \"$url\" -o dist/selected.tar.gz"));
    assert!(lines(download).any(|line| line == "sha256sum dist/selected.tar.gz | grep \"$digest\""));
    let assemble = run(job, "Assemble state-aware marketplace package without rebuilding")?;
    for state in ["legacy-public)", "candidate-proven)"] { assert!(lines(assemble).any(|line| line == state)); }
    assert!(lines(assemble).any(|line| line == "scripts/inspect-release-archive dist/codexy-marketplace-plugin.tar.gz \"$staged\""));
    assert!(!lines(assemble).any(|line| line.split_ascii_whitespace().take(2).eq(["cargo", "build"])));
    Ok(())
}

#[test]
fn contract_names_selected_and_candidate_release_identities() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let contract: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(root.join(".agents/plugins/release-publish-contract.json"))?)?;
    assert_eq!(contract["schema"], "codexy.internal.release-publish-contract.v1");
    assert_eq!(contract["version"], "1.2.2");
    assert_eq!(contract["bootstrap"]["selectedVersion"], "1.2.2");
    assert_eq!(contract["bootstrap"]["candidateVersion"], "1.3.0");
    assert_eq!(contract["runtime"]["platforms"], serde_json::json!(["darwin-arm64", "linux-x86_64"]));
    for path in [contract["bootstrap"]["publicationWorkflow"].as_str(), contract["runtime"]["candidateWorkflow"].as_str(), contract["runtime"]["activationWorkflow"].as_str()] { assert!(root.join(path.ok_or("workflow")?).is_file()); }
    Ok(())
}

#[test]
fn touched_loc_workflow_runs_for_all_pull_requests() -> Result<(), Box<dyn std::error::Error>> {
    let workflow = document("touched-loc-gate.yml")?;
    assert!(workflow["on"]["pull_request"].is_null());
    let steps = workflow["jobs"]["touched-loc"]["steps"].as_sequence().ok_or("steps")?;
    assert!(steps.iter().any(|step| step["with"]["fetch-depth"] == 0));
    assert!(steps.iter().any(|step| step["run"].as_str().is_some_and(|run| lines(run).any(|line| line.split_ascii_whitespace().any(|word| word == "--check-touched-loc")))));
    Ok(())
}

#[test]
fn release_changelog_formats_a_single_commit_range() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    for args in [["init"].as_slice(), ["config", "user.email", "codexy@example.com"].as_slice(), ["config", "user.name", "Codexy Test"].as_slice()] { git(temp.path(), args)?; }
    std::fs::write(temp.path().join("file.txt"), "before\n")?; git(temp.path(), &["add", "file.txt"])?; git(temp.path(), &["commit", "-m", "before release"])?; git(temp.path(), &["tag", "v0.1.0"])?;
    std::fs::write(temp.path().join("file.txt"), "after\n")?; git(temp.path(), &["add", "file.txt"])?; git(temp.path(), &["commit", "-m", "one change"])?; git(temp.path(), &["tag", "v0.2.0"])?;
    let output = Command::new(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/generate-release-changelog")).current_dir(temp.path()).args(["v0.2.0", "v0.1.0"]).output()?;
    assert!(output.status.success());
    let lines = String::from_utf8(output.stdout)?;
    assert!(lines.lines().any(|line| line == "Changes since v0.1.0:"));
    assert!(lines.lines().any(|line| line.starts_with("- one change (")));
    Ok(())
}

fn document(name: &str) -> Result<Value, Box<dyn std::error::Error>> { Ok(serde_yaml::from_str(&std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/workflows").join(name))?)?) }
fn run<'a>(job: &'a Value, name: &str) -> Result<&'a str, Box<dyn std::error::Error>> { job["steps"].as_sequence().and_then(|steps| steps.iter().find(|step| step["name"] == name)).and_then(|step| step["run"].as_str()).ok_or_else(|| "run".into()) }
fn lines(run: &str) -> impl Iterator<Item = &str> { run.lines().map(str::trim).filter(|line| !line.is_empty()) }
fn git(path: &std::path::Path, args: &[&str]) -> Result<(), Box<dyn std::error::Error>> { assert!(Command::new("git").current_dir(path).args(args).status()?.success()); Ok(()) }
