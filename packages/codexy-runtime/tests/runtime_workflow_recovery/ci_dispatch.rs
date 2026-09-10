use super::*;

#[test]
fn activation_dispatches_reusable_ci_for_the_frozen_pr_head()
-> Result<(), Box<dyn std::error::Error>> {
    let activation = workflow("runtime-activation.yml")?;
    let job = activation["jobs"]["open-activation-pr"]
        .as_mapping()
        .ok_or("activation job")?;
    assert_eq!(activation["permissions"]["actions"], "write");
    let steps = job["steps"].as_sequence().ok_or("activation steps")?;
    let dispatch = run(
        &activation,
        "open-activation-pr",
        "Dispatch required CI for exact activation head",
    )?;
    assert!(dispatch.contains("gh workflow run"));
    support::assert_structured_literals(
        dispatch,
        "exact-head CI dispatcher",
        &[
            "\"$RUNNER_TEMP/codexy-runtime-contract/scripts/select-runtime-activation-branch.sh\" --open-pr \"$GH_REPO\" \"$branch\"",
            "gh api --paginate",
            "gh run list --repo \"$GH_REPO\" --workflow \"$1\" --branch \"$branch\"",
            "select(.headSha == $sha and .headBranch == $branch",
            ".displayTitle == $title",
            "gh workflow run \"$workflow\" --repo \"$GH_REPO\" --ref \"$branch\"",
            "gh run watch \"$run_id\" --repo \"$GH_REPO\" --exit-status",
            "gh run view \"$run_id\" --repo \"$GH_REPO\"",
            "head_sha\\t",
            "base_sha",
        ],
    );
    assert!(!dispatch.contains("gh pr list --head"));
    assert!(!dispatch.contains("gh pr view \"$pr_number\""));
    assert!(!dispatch.contains("gh run rerun"));
    assert!(!dispatch.contains("git commit"));
    assert!(
        steps
            .iter()
            .position(|step| step["name"] == "Create exactly one activation pull request")
            < steps
                .iter()
                .position(|step| step["name"] == "Dispatch required CI for exact activation head")
    );
    assert_eq!(activation["permissions"]["contents"], "write");
    assert_eq!(activation["permissions"]["pull-requests"], "write");
    Ok(())
}

#[test]
fn activation_dispatch_path_matrix_matches_existing_workflow_filters()
-> Result<(), Box<dyn std::error::Error>> {
    let activation = workflow("runtime-activation.yml")?;
    let dispatch = run(
        &activation,
        "open-activation-pr",
        "Dispatch required CI for exact activation head",
    )?;
    for literal in [
        "plugins/codexy/*",
        "plugins/codexy-devtools/*",
        ".codex/*",
        ".agents/plugins/*",
        "scripts/*",
        ".github/workflows/plugin-runtime-binaries.yml",
        "packages/getcodexy/*",
        "plugins/codexy-github/*",
        ".agents/plugins/marketplace.json",
        "docs/getcodexy-component-installation.md",
        ".github/workflows/python-package.yml",
        ".github/workflows/bootstrap-package.yml",
        "workflows=(rust-test.yml language-lint.yml touched-loc-gate.yml)",
        "workflows+=(plugin-runtime-binaries.yml)",
        "workflows+=(python-package.yml)",
    ] {
        assert!(
            dispatch.contains(literal),
            "missing path/dispatch literal: {literal}"
        );
    }
    Ok(())
}

#[test]
fn ci_workflows_bind_manual_runs_to_the_requested_head_and_base()
-> Result<(), Box<dyn std::error::Error>> {
    for name in [
        "language-lint.yml",
        "plugin-runtime-binaries.yml",
        "python-package.yml",
        "touched-loc-gate.yml",
    ] {
        let document = workflow(name)?;
        let title = document["run-name"].as_str().ok_or("CI run identity")?;
        assert!(title.starts_with("CI ") && title.contains("inputs.head_sha"));
        assert!(title.contains("github.event.pull_request.head.sha"));
        if name == "touched-loc-gate.yml" {
            assert!(
                title.contains("inputs.base_sha")
                    && title.contains("github.event.pull_request.base.sha")
            );
        }
        assert_ne!(
            document["permissions"]["actions"], "write",
            "{name} must not dispatch workflows"
        );
        assert_eq!(
            document["on"]["workflow_dispatch"]["inputs"]["head_sha"]["required"], true,
            "{name}"
        );
        let jobs = document["jobs"].as_mapping().ok_or("jobs")?;
        let mut checked_out = 0;
        for job in jobs.values() {
            for step in job["steps"].as_sequence().ok_or("steps")? {
                if step["uses"] == "actions/checkout@v7" {
                    let checkout = step["with"]["ref"].as_str().ok_or("checkout ref")?;
                    assert!(
                        checkout.contains("inputs.head_sha"),
                        "{name} checkout is not dispatch-bound"
                    );
                    checked_out += 1;
                }
            }
        }
        assert!(checked_out > 0, "{name} has no checkout");
        let text = serde_yaml::to_string(&document)?;
        assert!(
            text.contains("EXPECTED_HEAD_SHA") || text.contains("HEAD_SHA"),
            "{name} lacks exact-head proof"
        );
    }
    let touched = workflow("touched-loc-gate.yml")?;
    assert_eq!(
        touched["on"]["workflow_dispatch"]["inputs"]["base_sha"]["required"],
        true
    );
    let touched_run = run(
        &touched,
        "touched-loc",
        "Validate touched implementation LOC",
    )?;
    support::assert_structured_literals(
        touched_run,
        "exact-base touched LOC dispatch",
        &["--base-ref \"$BASE_SHA\""],
    );
    Ok(())
}

#[test]
fn rust_dispatch_has_an_explicit_non_measurement_ci_mode() -> Result<(), Box<dyn std::error::Error>>
{
    let rust = workflow("rust-test.yml")?;
    let title = rust["run-name"].as_str().ok_or("Rust run identity")?;
    assert!(title.contains("inputs.run_mode == 'measurement' && 'Measurement' || 'CI'"));
    assert!(
        title.contains("inputs.head_sha") && title.contains("github.event.pull_request.head.sha")
    );
    let run_mode = &rust["on"]["workflow_dispatch"]["inputs"]["run_mode"];
    assert_eq!(run_mode["required"], true);
    assert_eq!(run_mode["options"][0], "measurement");
    assert_eq!(run_mode["options"][1], "ci");
    let text = serde_yaml::to_string(&rust)?;
    assert!(text.contains("inputs.run_mode == 'measurement'"));
    assert!(text.contains("inputs.run_mode == 'ci'"));
    assert!(text.contains("git rev-parse HEAD"));
    Ok(())
}
