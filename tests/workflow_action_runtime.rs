use serde_yaml::Value;

#[test]
fn workflows_use_current_node24_action_releases() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
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
            if version != expected {
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
    match value {
        Value::Mapping(mapping) => {
            for (key, value) in mapping {
                if key.as_str() == Some("uses") {
                    if let Some(action) = value.as_str() {
                        actions.push(action);
                    }
                }
                collect_action_references(value, actions);
            }
        }
        Value::Sequence(values) => {
            for value in values {
                collect_action_references(value, actions);
            }
        }
        _ => {}
    }
}

fn current_node24_version(action: &str) -> Option<&'static str> {
    match action {
        "actions/checkout" => Some("v7"),
        "actions/setup-python" => Some("v7"),
        "actions/upload-artifact" => Some("v7"),
        "actions/download-artifact" => Some("v8"),
        _ => None,
    }
}
