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
    let staging = open_pr
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
    for wrapper in [
        "plugins/codexy/mcp/codexy-mcp-lsp",
        "plugins/codexy/mcp/codexy-mcp-codegraph",
    ] {
        assert!(
            staging.split_ascii_whitespace().any(|argument| argument == wrapper),
            "version-bump staging omits {wrapper}"
        );
        let changed_areas = open_pr
            .split("cat >\"${body_file}\" <<EOF\n")
            .nth(1)
            .ok_or("missing generated pull-request body")?;
        assert!(
            changed_areas.lines().any(|line| line.trim() == format!("- {wrapper}")),
            "version-bump body omits {wrapper}"
        );
    }
    Ok(())
}

#[test]
fn python_package_workflow_binds_parity_to_publish_job()
-> Result<(), Box<dyn std::error::Error>> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let workflow = std::fs::read_to_string(root.join(".github/workflows/python-package.yml"))?;
    assert_python_package_workflow(&workflow)?;

    let command = "          test \"v${version}\" = \"$GITHUB_REF_NAME\"";
    let comment_only = format!("          echo \"parity removed\"\n          # {}", command.trim());
    let without_publish_parity = workflow.replacen(command, &comment_only, 1);
    let wrong_job = without_publish_parity.replacen(
        "      - uses: actions/upload-artifact@v4",
        &format!(
            "      - name: Misplaced parity proof\n        run: {}\n      - uses: actions/upload-artifact@v4",
            command.trim()
        ),
        1,
    );
    assert!(assert_python_package_workflow(&wrong_job).is_err());
    Ok(())
}

fn named_step_run<'a>(steps: &'a [Value], name: &str) -> Result<&'a str, &'static str> {
    steps
        .iter()
        .find(|step| step.get("name").and_then(Value::as_str) == Some(name))
        .and_then(|step| step.get("run"))
        .and_then(Value::as_str)
        .ok_or("named workflow step or run command missing")
}

fn assert_python_package_workflow(text: &str) -> Result<(), Box<dyn std::error::Error>> {
    let document: Value = serde_yaml::from_str(text)?;
    let root = document.as_mapping().ok_or("workflow root")?;
    let triggers = root
        .iter()
        .find(|(key, _)| key.as_str() == Some("on") || **key == Value::Bool(true))
        .and_then(|(_, value)| value.as_mapping())
        .ok_or("workflow triggers")?;
    let tags = triggers
        .get(Value::String("push".into()))
        .and_then(|push| push.get("tags"))
        .and_then(Value::as_sequence)
        .ok_or("push tags")?;
    if tags.as_slice() != [Value::String("v*".into())] {
        return Err("Python package trigger must accept reusable v* tags".into());
    }
    let jobs = root
        .get(Value::String("jobs".into()))
        .and_then(Value::as_mapping)
        .ok_or("workflow jobs")?;
    let publish = jobs
        .get(Value::String("publish".into()))
        .and_then(Value::as_mapping)
        .ok_or("publish job")?;
    if publish.get("if").and_then(Value::as_str)
        != Some("startsWith(github.ref, 'refs/tags/v')")
    {
        return Err("publish job must be tag-only".into());
    }
    let url = publish
        .get(Value::String("environment".into()))
        .and_then(|environment| environment.get("url"))
        .and_then(Value::as_str);
    if url != Some("https://pypi.org/p/getcodexy") {
        return Err("publish environment must target getcodexy".into());
    }
    let command = "test \"v${version}\" = \"$GITHUB_REF_NAME\"";
    let parity_jobs = jobs
        .iter()
        .filter_map(|(name, job)| Some((name.as_str()?, job.get("steps")?.as_sequence()?)))
        .filter(|(_, steps)| {
            steps.iter().any(|step| {
                step.get("run")
                    .and_then(Value::as_str)
                    .is_some_and(|run| run.lines().map(str::trim).any(|line| line == command))
            })
        })
        .map(|(name, _)| name)
        .collect::<Vec<_>>();
    if parity_jobs != ["publish"] {
        return Err(format!("tag parity command must run only in publish: {parity_jobs:?}").into());
    }
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
