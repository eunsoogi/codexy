use serde_yaml::Value;

#[test]
fn checks_tag_parity_before_release_mutations() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let workflow =
        std::fs::read_to_string(root.join(".github/workflows/plugin-runtime-binaries.yml"))?;
    assert_workflow_gate(&workflow)?;

    let without_gate = workflow.replace(
        "run: scripts/sync-plugin-version --check --tag \"$RELEASE_TAG\"",
        "run: echo parity gate removed",
    );
    assert!(assert_workflow_gate(&without_gate).is_err());
    let with_published_trigger = workflow.replacen(
        "  push:\n",
        "  release:\n    types: [published]\n  push:\n",
        1,
    );
    assert!(assert_workflow_gate(&with_published_trigger).is_err());
    Ok(())
}

fn assert_workflow_gate(text: &str) -> Result<(), Box<dyn std::error::Error>> {
    let document: Value = serde_yaml::from_str(text)?;
    let root = document.as_mapping().ok_or("workflow root")?;
    let triggers = root
        .iter()
        .find(|(key, _)| key.as_str() == Some("on") || **key == Value::Bool(true))
        .and_then(|(_, value)| value.as_mapping())
        .ok_or("workflow triggers")?;
    if triggers.contains_key(Value::String("release".into())) {
        return Err("published-release trigger bypasses pre-publication gate".into());
    }
    let jobs = root
        .get(Value::String("jobs".into()))
        .and_then(Value::as_mapping)
        .ok_or("jobs")?;
    let publish = jobs
        .get(Value::String("publish-release".into()))
        .and_then(Value::as_mapping)
        .ok_or("publish-release")?;
    let steps = publish
        .get(Value::String("steps".into()))
        .and_then(Value::as_sequence)
        .ok_or("publish steps")?;
    let parity = steps
        .iter()
        .position(|step| {
            step.get("run").and_then(Value::as_str)
                == Some("scripts/sync-plugin-version --check --tag \"$RELEASE_TAG\"")
        })
        .ok_or("missing executable parity step")?;
    for mutation in ["gh release create", "gh release edit", "gh release upload"] {
        let index = steps
            .iter()
            .position(|step| {
                step.get("run")
                    .and_then(Value::as_str)
                    .is_some_and(|run| run.contains(mutation))
            })
            .ok_or(mutation)?;
        if parity >= index {
            return Err(format!("parity gate follows {mutation}").into());
        }
    }
    Ok(())
}
