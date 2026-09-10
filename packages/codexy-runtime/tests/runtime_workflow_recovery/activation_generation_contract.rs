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
        } else {
            assert!(run.contains("branch=\"$(cat \"$branch_file\")\""));
            assert!(run.contains("git check-ref-format --branch \"$branch\""));
            assert!(!run.contains("select-runtime-activation-branch.sh"));
        }
    }
    let prepare = super::run(&activation, "open-activation-pr", "Prepare one version-selection branch")?;
    assert!(!prepare.contains("branch=\"codexy/runtime-activation-v${BOOTSTRAP_VERSION}\""));
    Ok(())
}
