use serde_yaml::Value;

use crate::support;

#[test]
fn publication_phases_are_separate_and_explicitly_gated() -> Result<(), Box<dyn std::error::Error>> {
    let bootstrap = document("bootstrap-package.yml")?;
    let candidate = document("runtime-candidate.yml")?;
    let activation = document("runtime-activation.yml")?;
    for workflow in [&bootstrap, &candidate, &activation] { assert_dispatch_only(workflow)?; }
    assert_eq!(bootstrap["jobs"]["publish-bootstrap"]["permissions"]["id-token"], "write");
    let bootstrap_proof = run(&bootstrap, "publish-bootstrap", "Prove public wheel and source distribution availability")?;
    for line in ["attempt=0", "test \"$attempt\" -lt 12 || exit 1", "for package_type in (\"bdist_wheel\", \"sdist\"):", "printf '%s  %s\\n' \"$digest\" \"public-${package_type}\" | sha256sum -c -"] {
        assert!(lines(bootstrap_proof).any(|actual| actual == line));
    }
    let candidate_assembly = run(&candidate, "publish-candidate", "Assemble canonical candidate archive and receipt")?;
    assert!(lines(candidate_assembly).any(|line| line == "rsync -a --exclude runtime --exclude runtime-release.json --exclude runtime-candidate.json plugins/codexy/ \"$root/\""));
    assert!(lines(candidate_assembly).any(|line| line == "slug=\"${CANDIDATE_TAG#runtime-candidate-}\""));
    assert!(lines(candidate_assembly).any(|line| line == "case \"$slug\" in *[!A-Za-z0-9._-]*) exit 1;; esac"));
    let copied = lines(candidate_assembly).position(|line| line == "cp -R staged-runtime \"$root/runtime\"").ok_or("candidate copy")?;
    let executable = lines(candidate_assembly).position(|line| line == "chmod 755 \"$root/runtime/codexy-mcp-${server}-${platform}.bin\"").ok_or("candidate mode")?;
    assert!(copied < executable);
    let candidate_publish = run(&candidate, "publish-candidate", "Create candidate tag and release once")?;
    assert!(command_present(candidate_publish, &["gh", "release", "create"]));
    assert!(!command_present(candidate_publish, &["gh", "release", "edit"]));
    let proof = step_index(&activation, "open-activation-pr", "Prove public bootstrap, release, run, and candidate bytes")?;
    let apply = step_index(&activation, "open-activation-pr", "Apply verified activation contract")?;
    let pr = step_index(&activation, "open-activation-pr", "Create activation pull request")?;
    assert!(proof < apply && apply < pr);
    let activation_proof = run(&activation, "open-activation-pr", "Prove public bootstrap, release, run, and candidate bytes")?;
    assert!(lines(activation_proof).any(|line| line == "jq -cS .candidate candidate-receipt.json | tr -d '\\n' > receipt-candidate.json"));
    assert!(command_present(activation_proof, &["gh", "attestation", "verify"]));
    let activation_pr = run(&activation, "open-activation-pr", "Create activation pull request")?;
    assert!(lines(activation_pr).any(|line| line.starts_with("git add ") && line.split_ascii_whitespace().any(|word| word == "plugins/codexy/runtime-candidate.json")));
    assert!(lines(activation_pr).any(|line| line.starts_with("git add ") && line.split_ascii_whitespace().any(|word| word == ".agents/plugins/release-publish-contract.json")));
    support::assert_structured_literals(
        activation_pr,
        "activation pull request metadata",
        &["--title \"feat(runtime): activate ${CANDIDATE_TAG}\"", "Closes #477", "Closes #451"],
    );
    Ok(())
}

#[test]
fn version_bump_stages_python_metadata() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
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
fn bootstrap_candidate_identity_is_independent_from_plugin_version() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let package: toml::Value = toml::from_str(&std::fs::read_to_string(root.join("packages/getcodexy/pyproject.toml"))?)?;
    let contract: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(root.join(".agents/plugins/release-publish-contract.json"))?)?;
    assert_eq!(package["project"]["version"].as_str(), Some("1.3.0"));
    assert_eq!(contract["version"], "1.2.2");
    assert_eq!(contract["bootstrap"]["selectedVersion"], "1.2.2");
    assert_eq!(contract["bootstrap"]["candidateVersion"], "1.3.0");
    assert_eq!(contract["runtime"]["selectedTag"], "v1.2.2");
    Ok(())
}

fn document(name: &str) -> Result<Value, Box<dyn std::error::Error>> { Ok(serde_yaml::from_str(&std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/workflows").join(name))?)?) }
fn lines(run: &str) -> impl Iterator<Item = &str> { run.lines().map(str::trim).filter(|line| !line.is_empty()) }
fn command_present(run: &str, words: &[&str]) -> bool { lines(run).any(|line| line.split_ascii_whitespace().collect::<Vec<_>>().windows(words.len()).any(|actual| actual == words)) }
fn assert_dispatch_only(value: &Value) -> Result<(), Box<dyn std::error::Error>> { let trigger = value.as_mapping().and_then(|root| root.iter().find(|(key, _)| key.as_str() == Some("on") || **key == Value::Bool(true))).and_then(|(_, value)| value.as_mapping()).ok_or("triggers")?; assert_eq!(trigger.len(), 1); assert!(trigger.contains_key(Value::String("workflow_dispatch".into()))); Ok(()) }
fn steps<'a>(value: &'a Value, job: &str) -> Result<&'a [Value], Box<dyn std::error::Error>> { value["jobs"][job]["steps"].as_sequence().map(Vec::as_slice).ok_or_else(|| "steps".into()) }
fn step_index(value: &Value, job: &str, name: &str) -> Result<usize, Box<dyn std::error::Error>> { steps(value, job)?.iter().position(|step| step["name"] == name).ok_or_else(|| "step".into()) }
fn run<'a>(value: &'a Value, job: &str, name: &str) -> Result<&'a str, Box<dyn std::error::Error>> { steps(value, job)?.iter().find(|step| step["name"] == name).and_then(|step| step["run"].as_str()).ok_or_else(|| "run".into()) }

fn named_step_run<'a>(steps: &'a [Value], name: &str) -> Result<&'a str, &'static str> {
    steps
        .iter()
        .find(|step| step.get("name").and_then(Value::as_str) == Some(name))
        .and_then(|step| step.get("run"))
        .and_then(Value::as_str)
        .ok_or("named workflow step or run command missing")
}
