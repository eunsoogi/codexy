use std::collections::BTreeSet;

use serde_yaml::{Mapping, Value};

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
fn version_bump_separates_target_and_published_bootstrap() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let workflow =
        std::fs::read_to_string(root.join(".github/workflows/plugin-version-bump.yml"))?;
    let document: Value = serde_yaml::from_str(&workflow)?;
    let inputs = workflow_inputs(&document)?;
    assert_eq!(inputs["advance_bootstrap"]["default"], false);
    assert_eq!(inputs["advance_bootstrap"]["required"], true);
    assert_eq!(inputs["advance_bootstrap"]["type"], "boolean");
    let jobs = document
        .get("jobs")
        .and_then(Value::as_mapping)
        .ok_or("workflow jobs")?;
    let availability = job_steps(jobs, "verify-published-bootstrap", "${{ inputs.advance_bootstrap }}", None)?;
    assert_eq!(single_url(named_step_run(availability, "Verify public PyPI bootstrap", None)?)?, "https://pypi.org/pypi/getcodexy/{version}/json");
    let steps = job_steps(jobs, "open-version-pr", "${{ always() && (!inputs.advance_bootstrap || needs.verify-published-bootstrap.result == 'success') }}", Some("verify-published-bootstrap"))?;
    assert_eq!(shell_lines(named_step_run(steps, "Synchronize target version", Some("${{ !inputs.advance_bootstrap }}"))?), ["scripts/sync-plugin-version --version \"$VERSION\""]);
    assert_eq!(shell_lines(named_step_run(steps, "Advance published bootstrap", Some("${{ inputs.advance_bootstrap }}"))?), ["scripts/sync-plugin-version --check --tag \"v$VERSION\"", "scripts/sync-plugin-version --advance-bootstrap"]);
    let open_pr = named_step_run(steps, "Open version bump pull request", None)?;
    let (bootstrap, ordinary) = staging_branches(open_pr)?;
    assert_eq!(staged_paths(&bootstrap)?, BTreeSet::from([".agents/plugins/release-publish-contract.json", "plugins/codexy/mcp/codexy-mcp-codegraph", "plugins/codexy/mcp/codexy-mcp-lsp"]));
    assert_eq!(staged_paths(&ordinary)?, BTreeSet::from([".agents/plugins/marketplace.json", ".agents/plugins/release-publish-contract.json", "Cargo.lock", "Cargo.toml", "package.json", "packages/getcodexy/pyproject.toml", "plugins/codexy/.codex-plugin/plugin.json"]));
    assert_eq!(pr_base(open_pr)?, "main");
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

fn named_step_run<'a>(steps: &'a [Value], name: &str, condition: Option<&str>) -> Result<&'a str, &'static str> {
    let matches = steps
        .iter()
        .filter(|step| step.get("name").and_then(Value::as_str) == Some(name) && condition.is_none_or(|value| step.get("if").and_then(Value::as_str) == Some(value)))
        .filter_map(|step| step.get("run").and_then(Value::as_str))
        .collect::<Vec<_>>();
    (matches.len() == 1)
        .then(|| matches[0])
        .ok_or("one named workflow step run required")
}

fn workflow_inputs(document: &Value) -> Result<&Value, &'static str> {
    document.as_mapping().and_then(|root| root.iter().find(|(key, _)| key.as_str() == Some("on") || **key == Value::Bool(true))).map(|(_, trigger)| trigger).and_then(|trigger| trigger.get("workflow_dispatch")).and_then(|dispatch| dispatch.get("inputs")).ok_or("version-bump inputs")
}

fn job_steps<'a>(jobs: &'a Mapping, name: &str, condition: &str, needs: Option<&str>) -> Result<&'a Vec<Value>, &'static str> {
    jobs.get(Value::String(name.into())).filter(|job| job.get("if").and_then(Value::as_str) == Some(condition) && needs.is_none_or(|need| job.get("needs").and_then(Value::as_str) == Some(need))).and_then(|job| job.get("steps")).and_then(Value::as_sequence).ok_or("workflow job contract")
}

fn shell_lines(run: &str) -> Vec<&str> {
    run.lines().map(|line| line.trim().trim_end_matches('\\').trim_end()).filter(|line| !line.is_empty() && !line.starts_with('#')).collect()
}

fn single_url(run: &str) -> Result<&str, &'static str> {
    let urls = shell_lines(run).into_iter().filter_map(|line| line.strip_prefix("url = f\"").and_then(|url| url.strip_suffix('"'))).collect::<Vec<_>>();
    (urls.len() == 1).then(|| urls[0]).ok_or("one public PyPI URL assignment required")
}

fn staging_branches(run: &str) -> Result<(Vec<&str>, Vec<&str>), &'static str> {
    let lines = shell_lines(run); let start = lines.iter().position(|line| *line == "if [ \"$ADVANCE_BOOTSTRAP\" = \"true\" ]; then").ok_or("bootstrap condition")?;
    let (mut depth, mut split) = (0, None); let mut end = None;
    for (index, line) in lines.iter().enumerate().skip(start + 1) { match *line { line if line.starts_with("if ") => depth += 1, "fi" if depth == 0 => { end = Some(index); break; }, "fi" => depth -= 1, "else" if depth == 0 => { if split.replace(index).is_some() { return Err("duplicate bootstrap else"); } }, _ => {} } }
    let split = split.ok_or("bootstrap else")?; let end = end.ok_or("bootstrap fi")?;
    Ok((lines[start + 1..split].to_vec(), lines[split + 1..end].to_vec()))
}

fn staged_paths<'a>(lines: &'a [&'a str]) -> Result<BTreeSet<&'a str>, &'static str> {
    let switched = lines
        .iter()
        .position(|line| *line == "git switch -c \"${branch}\"")
        .ok_or("staging branch switch")?;
    let paths = lines
        .iter()
        .enumerate()
        .filter_map(|(index, line)| (index > switched).then(|| line.strip_prefix("git add ")))
        .flatten()
        .flat_map(str::split_whitespace)
        .collect::<Vec<_>>();
    let set = paths.iter().copied().collect::<BTreeSet<_>>();
    (!set.is_empty() && set.len() == paths.len()).then_some(set).ok_or("duplicate or missing staged paths")
}

fn pr_base(run: &str) -> Result<&str, &'static str> {
    let lines = shell_lines(run); let start = lines.iter().position(|line| *line == "gh pr create").ok_or("gh pr create")?;
    let bases = lines[start + 1..].iter().filter_map(|line| line.strip_prefix("--base ")).collect::<Vec<_>>();
    (bases.len() == 1).then(|| bases[0]).ok_or("one gh pr base required")
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
