use serde_yaml::Value;

use crate::support;

#[path = "release_workflow_parity/publication_phases.rs"]
mod publication_phases;

#[test]
fn version_bump_stages_python_metadata() -> Result<(), Box<dyn std::error::Error>> {
    let root = codexy_runtime::paths::repository_root();
    let workflow = std::fs::read_to_string(root.join(".github/workflows/plugin-version-bump.yml"))?;
    let document: Value = serde_yaml::from_str(&workflow)?;
    let jobs = document
        .get("jobs")
        .and_then(Value::as_mapping)
        .ok_or("workflow jobs")?;
    let steps = jobs
        .get(Value::String("open-version-pr".into()))
        .and_then(|job| job.get("steps"))
        .and_then(Value::as_sequence)
        .ok_or("version-bump steps")?;
    let sync = named_step_run(steps, "Synchronize plugin version")?;
    assert_eq!(sync, "scripts/sync-plugin-version --version \"$VERSION\"");
    let open_pr = named_step_run(steps, "Open version bump pull request")?;
    assert_eq!(open_pr, "scripts/reconcile-version-pr");
    let adapter = std::fs::read_to_string(root.join(open_pr))?;
    let staging = adapter
        .lines()
        .map(str::trim)
        .find(|line| line.starts_with("git add "))
        .ok_or("missing version-bump staging command")?;
    assert!(
        staging
            .split_ascii_whitespace()
            .any(|argument| argument == "packages/getcodexy/pyproject.toml"),
        "version-bump staging omits Python metadata"
    );
    let admission = steps
        .iter()
        .position(|step| step["name"] == "Admit selected runtime version advance")
        .ok_or("version admission")?;
    let mutation = steps
        .iter()
        .position(|step| step["name"] == "Synchronize plugin version")
        .ok_or("version mutation")?;
    assert!(admission < mutation);
    assert_eq!(
        steps[admission]["run"],
        "scripts/sync-plugin-version --admit-version \"$VERSION\""
    );
    assert!(
        staging
            .split_ascii_whitespace()
            .any(|argument| argument == ".agents/plugins/release-publish-contract.json")
    );
    for excluded in ["runtime-release.json", "mcp/codexy-mcp"] {
        assert!(
            !staging
                .split_ascii_whitespace()
                .any(|argument| argument == excluded)
        );
    }
    Ok(())
}

#[test]
fn activated_bootstrap_identity_preserves_the_selected_runtime_release()
-> Result<(), Box<dyn std::error::Error>> {
    let root = codexy_runtime::paths::repository_root();
    let package: toml::Value = toml::from_str(&std::fs::read_to_string(
        root.join("packages/getcodexy/pyproject.toml"),
    )?)?;
    let contract: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(
        root.join(".agents/plugins/release-publish-contract.json"),
    )?)?;
    assert_eq!(package["project"]["version"].as_str(), Some("1.3.0"));
    assert_eq!(contract["version"], "1.3.0");
    assert_eq!(contract["bootstrap"]["selectedVersion"], "1.3.0");
    assert_eq!(contract["bootstrap"]["candidateVersion"], "1.3.0");
    assert_eq!(contract["runtime"]["selectedTag"], "v1.3.0");
    let runtime_release: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(
        root.join("plugins/codexy-devtools/runtime-release.json"),
    )?)?;
    assert_eq!(runtime_release["artifact"]["tag"], "v1.2.2");
    Ok(())
}

fn document(name: &str) -> Result<Value, Box<dyn std::error::Error>> {
    Ok(serde_yaml::from_str(&std::fs::read_to_string(
        codexy_runtime::paths::repository_root()
            .join(".github/workflows")
            .join(name),
    )?)?)
}
fn script(name: &str) -> Result<String, Box<dyn std::error::Error>> {
    Ok(std::fs::read_to_string(
        codexy_runtime::paths::repository_root()
            .join("scripts")
            .join(name),
    )?)
}
fn lines(run: &str) -> impl Iterator<Item = &str> {
    run.lines().map(str::trim).filter(|line| !line.is_empty())
}
fn command_present(run: &str, words: &[&str]) -> bool {
    lines(run).any(|line| {
        line.split_ascii_whitespace()
            .collect::<Vec<_>>()
            .windows(words.len())
            .any(|actual| actual == words)
    })
}
fn assert_dispatch_only(value: &Value) -> Result<(), Box<dyn std::error::Error>> {
    let trigger = value
        .as_mapping()
        .and_then(|root| {
            root.iter()
                .find(|(key, _)| key.as_str() == Some("on") || **key == Value::Bool(true))
        })
        .and_then(|(_, value)| value.as_mapping())
        .ok_or("triggers")?;
    assert_eq!(trigger.len(), 1);
    assert!(trigger.contains_key(Value::String("workflow_dispatch".into())));
    Ok(())
}
fn steps<'a>(value: &'a Value, job: &str) -> Result<&'a [Value], Box<dyn std::error::Error>> {
    value["jobs"][job]["steps"]
        .as_sequence()
        .map(Vec::as_slice)
        .ok_or_else(|| "steps".into())
}
fn step_index(value: &Value, job: &str, name: &str) -> Result<usize, Box<dyn std::error::Error>> {
    steps(value, job)?
        .iter()
        .position(|step| step["name"] == name)
        .ok_or_else(|| "step".into())
}
fn run<'a>(value: &'a Value, job: &str, name: &str) -> Result<&'a str, Box<dyn std::error::Error>> {
    steps(value, job)?
        .iter()
        .find(|step| step["name"] == name)
        .and_then(|step| step["run"].as_str())
        .ok_or_else(|| "run".into())
}

fn named_step_run<'a>(steps: &'a [Value], name: &str) -> Result<&'a str, &'static str> {
    steps
        .iter()
        .find(|step| step.get("name").and_then(Value::as_str) == Some(name))
        .and_then(|step| step.get("run"))
        .and_then(Value::as_str)
        .ok_or("named workflow step or run command missing")
}
