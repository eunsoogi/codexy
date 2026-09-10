#[test]
fn activation_steps_reuse_the_receipt_bound_generation_branch()
-> Result<(), Box<dyn std::error::Error>> {
    let activation = super::workflow("runtime-activation.yml")?;
    for step in [
        "Prepare one version-selection branch",
        "Apply verified activation and version-selection contract",
        "Stage and verify activation branch",
        "Create exactly one activation pull request",
        "Dispatch required CI for exact activation head",
    ] {
        let run = super::run(&activation, "open-activation-pr", step)?;
        assert!(run.contains("branch_file=\"$RUNNER_TEMP/codexy-runtime-activation-branch\""));
        if step == "Prepare one version-selection branch" {
            assert!(run.contains("select-runtime-activation-branch.sh"));
            assert!(run.contains("runtime-staging-receipt.json"));
            assert!(run.contains("printf '%s\\n' \"$branch\" > \"$branch_file\""));
        } else if matches!(
            step,
            "Apply verified activation and version-selection contract"
                | "Stage and verify activation branch"
        ) {
            assert!(run.contains("branch=\"$(cat \"$branch_file\")\""));
            assert!(run.contains("git check-ref-format --branch \"$branch\""));
            assert!(!run.contains("select-runtime-activation-branch.sh"));
        } else {
            assert!(run.contains("--open-pr"));
        }
    }
    let prepare = super::run(
        &activation,
        "open-activation-pr",
        "Prepare one version-selection branch",
    )?;
    assert!(!prepare.contains("branch=\"codexy/runtime-activation-v${BOOTSTRAP_VERSION}\""));
    Ok(())
}

#[test]
fn activation_pr_identity_checks_cover_every_downstream_consumer()
-> Result<(), Box<dyn std::error::Error>> {
    let selector = super::script("select-runtime-activation-branch.sh")?;
    let activation = super::workflow("runtime-activation.yml")?;
    for literal in [
        "--json number,headRefName,headRefOid,baseRefName,baseRefOid,isCrossRepository,headRepository,headRepositoryOwner",
        ".isCrossRepository == false",
        ".headRepository.nameWithOwner == $repository",
        ".headRepositoryOwner.login == $repository_owner",
        ".baseRefName != \"main\"",
        ".baseRefOid | type",
    ] {
        assert!(selector.contains(literal), "selector lacks identity guard: {literal}");
    }
    for (step, repository) in [
        (
            "Create exactly one activation pull request",
            "\"$GITHUB_REPOSITORY\"",
        ),
        (
            "Dispatch required CI for exact activation head",
            "\"$GH_REPO\"",
        ),
    ] {
        let run = super::run(&activation, "open-activation-pr", step)?;
        assert!(
            run.contains(&format!(
                "\"$RUNNER_TEMP/codexy-runtime-contract/scripts/select-runtime-activation-branch.sh\" --open-pr {repository} \"$branch\""
            )),
            "{step} does not consume the identity-checked selector"
        );
    }
    Ok(())
}
