use std::{fs, path::{Path, PathBuf}, process::{Command, Output}};

use serde_yaml::Value;

use crate::support;

#[test]
fn publication_phases_are_separate_and_explicitly_gated() -> Result<(), Box<dyn std::error::Error>> {
    let bootstrap = document("bootstrap-package.yml")?;
    let staging = document("runtime-candidate.yml")?;
    let activation = document("runtime-activation.yml")?;
    let publisher = document("publish-version-release.yml")?;
    for workflow in [&bootstrap, &staging, &activation, &publisher] { assert_dispatch_only(workflow)?; }
    assert_eq!(bootstrap["jobs"]["publish-bootstrap"]["permissions"]["id-token"], "write");
    let bootstrap_proof = run(&bootstrap, "publish-bootstrap", "Prove public wheel and source distribution availability")?;
    for line in ["attempt=0", "test \"$attempt\" -lt 12 || exit 1", "for package_type in (\"bdist_wheel\", \"sdist\"):", "printf '%s  %s\\n' \"$digest\" \"public-${package_type}\" | sha256sum -c -"] {
        assert!(lines(bootstrap_proof).any(|actual| actual == line));
    }
    let bootstrap_clean_index = step_index(&bootstrap, "publish-bootstrap", "Prove clean public bootstrap install")?;
    assert!(step_index(&bootstrap, "publish-bootstrap", "Prove public wheel and source distribution availability")? < bootstrap_clean_index);
    let bootstrap_clean = run(&bootstrap, "publish-bootstrap", "Prove clean public bootstrap install")?;
    for required in [
        "simple_index_attempt=0",
        "https://pypi.org/simple/getcodexy/",
        "--max-time 20",
        "BOOTSTRAP_VERSION=\"$BOOTSTRAP_VERSION\" python3 - simple-index.html <<'PY'",
        "version_prefix = f\"getcodexy-{version}\"",
        "if test \"$simple_index_attempt\" -ge 12; then",
        "refusing exact-version install",
        "python -m venv public-bootstrap",
        "public-bootstrap/bin/python -m pip install --index-url https://pypi.org/simple \"getcodexy==${BOOTSTRAP_VERSION}\"",
        "public-bootstrap/bin/codexy-mcp-runtime --help",
    ] {
        assert!(bootstrap_clean.contains(required), "missing clean-install propagation contract: {required}");
    }
    assert!(!bootstrap_clean.contains("pip install --retries"));
    assert!(bootstrap_clean.find("python -m venv public-bootstrap").unwrap() < bootstrap_clean.find("pip install --index-url").unwrap());
    assert!(bootstrap_clean.find("pip install --index-url").unwrap() < bootstrap_clean.find("codexy-mcp-runtime --help").unwrap());
    let staging_assembly = run(&staging, "stage-runtime", "Assemble canonical staged archive and receipt")?;
    assert_eq!(staging_assembly, "scripts/assemble-runtime-candidate");
    let staging_assembly = script("assemble-runtime-candidate")?;
    assert!(lines(&staging_assembly).any(|line| line == "rsync -a --exclude runtime --exclude runtime-release.json --exclude runtime-candidate.json plugins/codexy-devtools/ \"$root/\""));
    let copied = lines(&staging_assembly).position(|line| line == "cp -R staged-runtime \"$root/runtime\"").ok_or("staging copy")?;
    let executable = lines(&staging_assembly).position(|line| line == "chmod 755 \"$root/runtime/codexy-mcp-${server}-${platform}.bin\"").ok_or("staging mode")?;
    assert!(copied < executable);
    let proof = step_index(&activation, "open-activation-pr", "Prove public bootstrap and authenticated staging identity")?;
    let apply = step_index(&activation, "open-activation-pr", "Apply verified activation and version-selection contract")?;
    let pr = step_index(&activation, "open-activation-pr", "Create exactly one activation pull request")?;
    assert!(proof < apply && apply < pr);
    assert!(run(&activation, "open-activation-pr", "Apply verified activation and version-selection contract")?
        .contains("scripts/sync-plugin-version.sh --version \"$BOOTSTRAP_VERSION\""));
    let activation_proof = run(&activation, "open-activation-pr", "Prove public bootstrap and authenticated staging identity")?;
    assert!(lines(activation_proof).any(|line| line == "scripts/download-runtime-staging-artifact staging"));
    assert!(command_present(activation_proof, &["gh", "attestation", "verify"]));
    let activation_pr = run(&activation, "open-activation-pr", "Create exactly one activation pull request")?;
    assert!(lines(activation_pr).any(|line| line.starts_with("git add ") && line.split_ascii_whitespace().any(|word| word == "plugins/codexy-devtools")));
    assert!(lines(activation_pr).any(|line| line.starts_with("git add ") && line.split_ascii_whitespace().any(|word| word == ".agents/plugins")));
    assert!(lines(activation_pr).any(|line| line.starts_with("git add ") && line.split_ascii_whitespace().any(|word| word == "packages/getcodexy/src/codexy_runtime_tools/component-manifest.json")));
    assert!(lines(activation_pr).any(|line| line.starts_with("git add ") && line.split_ascii_whitespace().any(|word| word == "packages/getcodexy/uv.lock")));
    support::assert_structured_literals(
        activation_pr,
        "activation pull request metadata",
        &["--title \"feat(runtime): activate v${BOOTSTRAP_VERSION}\"", "version ${BOOTSTRAP_VERSION}"],
    );
    support::assert_structured_absent_literals(
        activation_pr,
        "activation pull request metadata",
        &["Fixes #"],
    );
    let release = run(&publisher, "publish-release", "Create and verify the only public version release")?;
    assert_eq!(release, "scripts/publish-verified-release");
    Ok(())
}

#[test]
fn clean_bootstrap_preflight_exercises_visibility_and_failure_boundaries() -> Result<(), Box<dyn std::error::Error>> {
    let bootstrap = document("bootstrap-package.yml")?;
    let clean = run(&bootstrap, "publish-bootstrap", "Prove clean public bootstrap install")?;
    let package: toml::Value = toml::from_str(&fs::read_to_string(codexy_runtime::paths::repository_root().join("packages/getcodexy/pyproject.toml"))?)?;
    let version = package["project"]["version"].as_str().ok_or("package version")?;
    let exact_index = format!("<a href=\"https://files.pythonhosted.org/getcodexy-{version}-py3-none-any.whl\">getcodexy-{version}-py3-none-any.whl</a>\n<a href=\"https://files.pythonhosted.org/getcodexy-{version}.tar.gz\">getcodexy-{version}.tar.gz</a>");
    let adjacent_index = format!("<a href=\"https://files.pythonhosted.org/getcodexy-{version}.post1-py3-none-any.whl\">getcodexy-{version}.post1-py3-none-any.whl</a>");
    let stale_root = tempfile::tempdir()?;
    let stale_curl = format!("count=0; test -f simple-index-attempts && count=$(cat simple-index-attempts); count=$((count + 1)); printf '%s\\n' \"$count\" > simple-index-attempts; if test \"$count\" -eq 1; then return 7; fi; printf '%s\\n' '{adjacent_index}' > simple-index.html");
    let stale = run_clean_preflight(clean, stale_root.path(), version, &stale_curl)?;
    assert_eq!(fs::read_to_string(stale_root.path().join("simple-index-attempts"))?.trim(), "12");
    assert_eq!(stale.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&stale.stderr).contains("after 12 bounded checks"));
    let positive_root = tempfile::tempdir()?;
    let positive_curl = format!("printf '%s\\n' '{exact_index}' > simple-index.html");
    let positive = run_clean_preflight(clean, positive_root.path(), version, &positive_curl)?;
    assert!(positive.status.success());
    let transport_root = tempfile::tempdir()?;
    let transport_curl = format!("count=0; test -f simple-index-attempts && count=$(cat simple-index-attempts); count=$((count + 1)); printf '%s\\n' \"$count\" > simple-index-attempts; if test \"$count\" -lt 2; then return 7; fi; printf '%s\\n' '{exact_index}' > simple-index.html");
    let transport = run_clean_preflight(clean, transport_root.path(), version, &transport_curl)?;
    assert!(transport.status.success() && fs::read_to_string(transport_root.path().join("simple-index-attempts"))?.trim() == "2" && String::from_utf8_lossy(&transport.stdout).contains("exposes getcodexy=="));
    Ok(())
}

#[test]
fn version_bump_stages_python_metadata() -> Result<(), Box<dyn std::error::Error>> {
    let root = codexy_runtime::paths::repository_root();
    let workflow =
        std::fs::read_to_string(root.join(".github/workflows/plugin-version-bump.yml"))?;
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
    assert_eq!(sync, "scripts/sync-plugin-version.sh --prepare-candidate \"$VERSION\"");
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
    assert_eq!(
        steps[admission]["run"],
        "scripts/sync-plugin-version.sh --admit-candidate \"$VERSION\""
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
fn bootstrap_identity_supports_selected_and_candidate_prepared_states() -> Result<(), Box<dyn std::error::Error>> {
    let root = codexy_runtime::paths::repository_root();
    let package: toml::Value = toml::from_str(&std::fs::read_to_string(root.join("packages/getcodexy/pyproject.toml"))?)?;
    let contract: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(root.join(".agents/plugins/release-publish-contract.json"))?)?;
    let package_version = package["project"]["version"].as_str().ok_or("package version")?;
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
    let bootstrap = std::fs::read_to_string(root.join("packages/codexy-runtime/src/version/bootstrap.rs"))?;
    let selected_constant = bootstrap
        .lines()
        .find_map(|line| line.strip_prefix("pub(super) const VERSION: &str = \"")?.strip_suffix("\";"))
        .ok_or("VERSION constant")?;
    let candidate_constant = bootstrap
        .lines()
        .find_map(|line| line.strip_prefix("pub(super) const CANDIDATE_VERSION: &str = \"")?.strip_suffix("\";"))
        .ok_or("CANDIDATE_VERSION constant")?;
    assert_eq!(selected_constant, selected_version);
    assert_eq!(candidate_constant, candidate_version);
    Ok(())
}

fn document(name: &str) -> Result<Value, Box<dyn std::error::Error>> { Ok(serde_yaml::from_str(&std::fs::read_to_string(codexy_runtime::paths::repository_root().join(".github/workflows").join(name))?)?) }
fn script(name: &str) -> Result<String, Box<dyn std::error::Error>> { Ok(std::fs::read_to_string(codexy_runtime::paths::repository_root().join("scripts").join(name))?) }
fn lines(run: &str) -> impl Iterator<Item = &str> { run.lines().map(str::trim).filter(|line| !line.is_empty()) }
fn command_present(run: &str, words: &[&str]) -> bool { lines(run).any(|line| line.split_ascii_whitespace().collect::<Vec<_>>().windows(words.len()).any(|actual| actual == words)) }
fn assert_dispatch_only(value: &Value) -> Result<(), Box<dyn std::error::Error>> { let trigger = value.as_mapping().and_then(|root| root.iter().find(|(key, _)| key.as_str() == Some("on") || **key == Value::Bool(true))).and_then(|(_, value)| value.as_mapping()).ok_or("triggers")?; assert_eq!(trigger.len(), 1); assert!(trigger.contains_key(Value::String("workflow_dispatch".into()))); Ok(()) }
fn steps<'a>(value: &'a Value, job: &str) -> Result<&'a [Value], Box<dyn std::error::Error>> { value["jobs"][job]["steps"].as_sequence().map(Vec::as_slice).ok_or_else(|| "steps".into()) }
fn step_index(value: &Value, job: &str, name: &str) -> Result<usize, Box<dyn std::error::Error>> { steps(value, job)?.iter().position(|step| step["name"] == name).ok_or_else(|| "step".into()) }
fn run<'a>(value: &'a Value, job: &str, name: &str) -> Result<&'a str, Box<dyn std::error::Error>> { steps(value, job)?.iter().find(|step| step["name"] == name).and_then(|step| step["run"].as_str()).ok_or_else(|| "run".into()) }
fn run_clean_preflight(run: &str, root: &Path, version: &str, curl_body: &str) -> Result<Output, Box<dyn std::error::Error>> {
    let preflight = run.splitn(2, "\npython -m venv public-bootstrap").next().ok_or("clean-install preflight")?;
    let script = format!("curl() {{ {curl_body}; }}\nsleep() {{ :; }}\n{preflight}");
    Ok(bash_command()?.args(["-euo", "pipefail", "-c", &script]).current_dir(root).env("BOOTSTRAP_VERSION", version).output()?)
}

fn bash_command() -> Result<Command, Box<dyn std::error::Error>> {
    let bash = if cfg!(windows) {
        let mut path = std::env::split_paths(&std::env::var_os("PATH").ok_or("PATH")?).collect::<Vec<_>>();
        for (variable, append_git) in [("GIT_INSTALL_ROOT", false), ("ProgramFiles", true), ("ProgramFiles(x86)", true)] {
            if let Some(root) = std::env::var_os(variable) {
                let git = append_git.then(|| PathBuf::from(&root).join("Git")).unwrap_or_else(|| PathBuf::from(root));
                path.extend([git.join("bin"), git.join("usr").join("bin")]);
            }
        }
        let path = std::env::join_paths(path)?;
        let extensions = std::env::var_os("PATHEXT").unwrap_or_else(|| ".COM;.EXE;.BAT;.CMD".into());
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
