use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_yaml::Value;

use crate::support;

#[path = "release_workflow_parity/publication.rs"]
mod publication;

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
    let sync = named_step_run(steps, "Prepare candidate plugin version")?;
    assert!(sync.contains("scripts/sync-plugin-version.sh --prepare-candidate"));
    let open_pr = named_step_run(steps, "Open version bump pull request")?;
    assert!(open_pr.contains("scripts/reconcile-version-pr"));
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
    assert!(
        staging
            .split_ascii_whitespace()
            .any(|argument| argument == "packages/getcodexy/uv.lock"),
        "version-bump staging omits the canonical Python version lock"
    );
    let admission = steps
        .iter()
        .position(|step| step["name"] == "Admit candidate version preparation")
        .ok_or("version admission")?;
    let mutation = steps
        .iter()
        .position(|step| step["name"] == "Prepare candidate plugin version")
        .ok_or("version mutation")?;
    assert!(admission < mutation);
    assert!(
        steps[admission]["run"]
            .as_str()
            .is_some_and(|run| run.contains("scripts/sync-plugin-version.sh --admit-candidate"))
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
fn bootstrap_identity_supports_selected_and_candidate_prepared_states()
-> Result<(), Box<dyn std::error::Error>> {
    let root = codexy_runtime::paths::repository_root();
    let package: toml::Value = toml::from_str(&std::fs::read_to_string(
        root.join("packages/getcodexy/pyproject.toml"),
    )?)?;
    let contract: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(
        root.join(".agents/plugins/release-publish-contract.json"),
    )?)?;
    let package_version = package["project"]["version"]
        .as_str()
        .ok_or("package version")?;
    let selected_version = contract["version"].as_str().ok_or("selected version")?;
    let candidate_version = contract["bootstrap"]["candidateVersion"]
        .as_str()
        .ok_or("candidate version")?;
    let runtime_release: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(
        root.join("plugins/codexy-devtools/runtime-release.json"),
    )?)?;
    let selected_runtime_tag = runtime_release["artifact"]["tag"]
        .as_str()
        .ok_or("selected runtime tag")?;
    assert_eq!(contract["bootstrap"]["selectedVersion"], selected_version);
    let expected_runtime_tag = if package_version == selected_version {
        format!("v{selected_version}")
    } else {
        selected_runtime_tag.to_owned()
    };
    assert_eq!(
        contract["runtime"]["selectedTag"].as_str(),
        Some(expected_runtime_tag.as_str())
    );
    assert_eq!(candidate_version, package_version);
    if package_version == selected_version {
        assert_eq!(candidate_version, selected_version);
    } else {
        assert_ne!(candidate_version, selected_version);
    }
    let bootstrap =
        std::fs::read_to_string(root.join("packages/codexy-runtime/src/version/bootstrap.rs"))?;
    let selected_constant = bootstrap
        .lines()
        .find_map(|line| {
            line.strip_prefix("pub(super) const VERSION: &str = \"")?
                .strip_suffix("\";")
        })
        .ok_or("VERSION constant")?;
    let candidate_constant = bootstrap
        .lines()
        .find_map(|line| {
            line.strip_prefix("pub(super) const CANDIDATE_VERSION: &str = \"")?
                .strip_suffix("\";")
        })
        .ok_or("CANDIDATE_VERSION constant")?;
    assert_eq!(selected_constant, selected_version);
    assert_eq!(candidate_constant, candidate_version);
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
fn assert_expected_triggers(
    value: &Value,
    has_pull_request: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let trigger = value
        .as_mapping()
        .and_then(|root| {
            root.iter()
                .find(|(key, _)| key.as_str() == Some("on") || **key == Value::Bool(true))
        })
        .and_then(|(_, value)| value.as_mapping())
        .ok_or("triggers")?;
    assert_eq!(trigger.len(), 1 + has_pull_request as usize);
    assert!(trigger.contains_key(Value::String("workflow_dispatch".into())));
    let pull_request = trigger.contains_key(Value::String("pull_request".into()));
    assert_eq!(pull_request, has_pull_request);
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
fn run_clean_preflight(
    run: &str,
    root: &Path,
    version: &str,
    curl_body: &str,
) -> Result<Output, Box<dyn std::error::Error>> {
    let preflight = run
        .splitn(2, "\npython -m venv public-bootstrap")
        .next()
        .ok_or("clean-install preflight")?;
    let script = format!("curl() {{ {curl_body}; }}\nsleep() {{ :; }}\n{preflight}");
    Ok(bash_command()?
        .args(["-euo", "pipefail", "-c", &script])
        .current_dir(root)
        .env("BOOTSTRAP_VERSION", version)
        .output()?)
}

fn bash_command() -> Result<Command, Box<dyn std::error::Error>> {
    let bash = if cfg!(windows) {
        let mut path =
            std::env::split_paths(&std::env::var_os("PATH").ok_or("PATH")?).collect::<Vec<_>>();
        for (variable, append_git) in [
            ("GIT_INSTALL_ROOT", false),
            ("ProgramFiles", true),
            ("ProgramFiles(x86)", true),
        ] {
            if let Some(root) = std::env::var_os(variable) {
                let git = append_git
                    .then(|| PathBuf::from(&root).join("Git"))
                    .unwrap_or_else(|| PathBuf::from(root));
                path.extend([git.join("bin"), git.join("usr").join("bin")]);
            }
        }
        let path = std::env::join_paths(path)?;
        let extensions =
            std::env::var_os("PATHEXT").unwrap_or_else(|| ".COM;.EXE;.BAT;.CMD".into());
        support::executable_path::executable_path_in("bash", &path, &extensions)?
    } else {
        PathBuf::from("bash")
    };
    Ok(Command::new(bash))
}

fn named_step_run<'a>(steps: &'a [Value], name: &str) -> Result<&'a str, &'static str> {
    steps
        .iter()
        .find(|step| step.get("name").and_then(Value::as_str) == Some(name))
        .and_then(|step| step.get("run"))
        .and_then(Value::as_str)
        .ok_or("named workflow step or run command missing")
}
