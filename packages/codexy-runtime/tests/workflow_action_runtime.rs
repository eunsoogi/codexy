use serde_yaml::Value;

#[test]
fn workflows_use_current_node24_action_releases() -> Result<(), Box<dyn std::error::Error>> {
    let root = codexy_runtime::paths::repository_root();
    assert_workflows_use_current_node24_action_releases(&root.join(".github/workflows"))
}

#[test]
fn runtime_audit_ignores_comments_and_strings() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    std::fs::write(
        temp.path().join("comment-decoy.yml"),
        "name: decoy\n# actions/checkout@v7\njobs:\n  check:\n    runs-on: ubuntu-latest\n    steps:\n      - run: echo actions/checkout@v7\n      - uses: actions/checkout@v4\n",
    )?;

    assert!(
        assert_workflows_use_current_node24_action_releases(temp.path()).is_err(),
        "a comment or shell string must not mask an obsolete uses reference"
    );
    Ok(())
}

#[test]
fn runtime_audit_discovers_each_workflow_file() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    std::fs::write(
        temp.path().join("additional-workflow.yaml"),
        "name: additional\njobs:\n  check:\n    runs-on: ubuntu-latest\n    steps:\n      - uses: actions/download-artifact@v4\n",
    )?;

    assert!(
        assert_workflows_use_current_node24_action_releases(temp.path()).is_err(),
        "every workflow file must be included in the runtime audit"
    );
    Ok(())
}

#[test]
fn runtime_audit_rejects_obsolete_attestation_action() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    std::fs::write(
        temp.path().join("attestation.yml"),
        "name: attest\njobs:\n  publish:\n    runs-on: ubuntu-latest\n    steps:\n      - uses: actions/attest-build-provenance@v2\n",
    )?;

    assert!(assert_workflows_use_current_node24_action_releases(temp.path()).is_err());
    Ok(())
}

#[test]
fn runtime_audit_ignores_non_step_uses_values() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    std::fs::write(
        temp.path().join("data.yml"),
        "name: data\njobs:\n  check:\n    runs-on: ubuntu-latest\n    env:\n      uses: actions/checkout@v4\n    steps:\n      - run: echo safe\n",
    )?;

    assert!(assert_workflows_use_current_node24_action_releases(temp.path()).is_ok());
    Ok(())
}

fn assert_workflows_use_current_node24_action_releases(
    workflows: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    for entry in std::fs::read_dir(workflows)? {
        let path = entry?.path();
        if !matches!(path.extension().and_then(std::ffi::OsStr::to_str), Some("yml" | "yaml")) {
            continue;
        }
        let workflow = std::fs::read_to_string(&path)?;
        let document: Value = serde_yaml::from_str(&workflow)?;
        let mut actions = Vec::new();
        collect_action_references(&document, &mut actions);
        for action in actions {
            let Some((name, version)) = action.split_once('@') else {
                continue;
            };
            let Some(expected) = current_node24_version(name) else {
                continue;
            };
            if version != expected && version != pinned_reference(name) {
                return Err(format!(
                    "{} uses {action}; expected {name}@{expected}",
                    path.display()
                )
                .into());
            }
        }
    }
    Ok(())
}

fn collect_action_references<'a>(value: &'a Value, actions: &mut Vec<&'a str>) {
    let Some(jobs) = value["jobs"].as_mapping() else {
        return;
    };
    for job in jobs.values() {
        if let Some(action) = job["uses"].as_str() {
            actions.push(action);
        }
        let Some(steps) = job["steps"].as_sequence() else {
            continue;
        };
        for step in steps {
            if let Some(action) = step["uses"].as_str() {
                actions.push(action);
            }
        }
    }
}

fn current_node24_version(action: &str) -> Option<&'static str> {
    match action {
        "actions/checkout" => Some("v7"),
        "actions/setup-python" => Some("v7"),
        "actions/upload-artifact" => Some("v7"),
        "actions/download-artifact" => Some("v8"),
        "actions/attest-build-provenance" => Some("v4"),
        _ => None,
    }
}

fn pinned_reference(action: &str) -> &'static str {
    match action {
        "actions/checkout" => "3d3c42e5aac5ba805825da76410c181273ba90b1",
        "actions/setup-python" => "5fda3b95a4ea91299a34e894583c3862153e4b97",
        "actions/upload-artifact" => "043fb46d1a93c77aae656e7c1c64a875d1fc6a0a",
        "actions/download-artifact" => "3e5f45b2cfb9172054b4087a40e8e0b5a5461e7c",
        "actions/attest-build-provenance" => "4d101475d8b20a2381f78447822ac1eab6504dd8",
        _ => "",
    }
}
