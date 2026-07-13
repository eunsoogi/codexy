use std::fs;

mod support;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn validator_requires_child_external_gate_and_archive_preflight_policy() -> TestResult {
    for (required, replacement, error_fragment) in [
        (
            "child external-gate wait MUST retain active goal and plan",
            "A child\n  external-gate wait is optional",
            "child external-gate wait must retain active goal and plan",
        ),
        (
            "inspect archive candidates and the active reservation ledger",
            "inspect archive candidates only when convenient",
            "inspect archive candidates and the active reservation ledger",
        ),
        (
            "MAY archive only terminal, unreferenced, clean and unreserved worktree lanes with no open PR or pending gate",
            "MAY archive any completed worktree lane",
            "may archive only terminal, unreferenced, clean and unreserved worktree lanes with no open pr or pending gate",
        ),
        (
            "After Sentinel BLOCK, the usable existing owner MUST record the `block` and update the plan to a repair step",
            "After Sentinel BLOCK, the usable existing owner MAY finish the lane",
            "usable existing owner must record the block and update the plan to a repair step",
        ),
        (
            "add faithful RED coverage, repair, rerun terminal proof, then invoke exactly one fresh Sentinel review for the new file state or head",
            "repair whenever convenient",
            "invoke exactly one fresh sentinel review for the new file state or head",
        ),
    ] {
        let (_temp, plugin_root) = support::copy_plugin_fixture()?;
        let path = plugin_root.join("skills/codex-orchestration/SKILL.md");
        let original = fs::read_to_string(&path)?;
        fs::write(&path, original.replace(required, replacement))?;

        let output = support::validator(&plugin_root, "--check")?;
        assert!(
            !output.status.success(),
            "validator accepted missing policy {required:?}"
        );
        assert!(support::stderr(&output).contains(error_fragment));
    }
    Ok(())
}

#[test]
fn validator_rejects_blocked_goal_or_replacement_thread_policy() -> TestResult {
    for forbidden in [
        "MUST call update_goal(status=\"blocked\") after a Sentinel BLOCK.",
        "MUST create a replacement thread after a Sentinel BLOCK.",
    ] {
        let (_temp, plugin_root) = support::copy_plugin_fixture()?;
        let path = plugin_root.join("skills/codex-orchestration/SKILL.md");
        fs::write(
            &path,
            format!("{}\n{forbidden}\n", fs::read_to_string(&path)?),
        )?;

        let output = support::validator(&plugin_root, "--check")?;
        assert!(
            !output.status.success(),
            "validator accepted forbidden BLOCK policy {forbidden:?}"
        );
        assert!(support::stderr(&output).contains("must not"));
    }
    Ok(())
}

#[test]
fn validator_ignores_historical_sections_for_required_and_forbidden_policy() -> TestResult {
    let (_temp, plugin_root) = support::copy_plugin_fixture()?;
    let path = plugin_root.join("skills/codex-orchestration/SKILL.md");
    let original = fs::read_to_string(&path)?;
    fs::write(
        &path,
        original.replace(
            "The root/orchestrator MAY end its goal and plan after dispatch; child external-gate wait MUST retain active goal and plan",
            "The root/orchestrator MAY end its goal and plan after dispatch.\n\n## Historical Example\nThis is retained only for historical context.\nA child\n  external-gate wait MUST retain active goal and plan",
        ),
    )?;

    let required_only_historically = support::validator(&plugin_root, "--check")?;
    assert!(!required_only_historically.status.success());
    assert!(
        support::stderr(&required_only_historically)
            .contains("child external-gate wait must retain active goal and plan")
    );

    fs::write(
        &path,
        format!(
            "{original}\n## Historical Example\nThis is retained only for historical context.\nMUST keep polling and keep the goal active.\n"
        ),
    )?;
    let forbidden_only_historically = support::validator(&plugin_root, "--check")?;
    assert!(
        forbidden_only_historically.status.success(),
        "{}",
        support::stderr(&forbidden_only_historically)
    );

    fs::write(
        &path,
        format!(
            "{original}\nLogging is not required, but MUST keep polling and keep the goal active.\n"
        ),
    )?;
    let unrelated_negation = support::validator(&plugin_root, "--check")?;
    assert!(!unrelated_negation.status.success());
    assert!(support::stderr(&unrelated_negation).contains("autonomous polling"));

    fs::write(
        &path,
        original.replace(
            "The root/orchestrator MAY end its goal and plan after dispatch; child external-gate wait MUST retain active goal and plan, use bounded child-local monitoring, and send a parent delta before transition.",
            "The root/orchestrator MAY end its goal and plan after dispatch.\n\n## Child external-gate wait MUST retain active goal and plan",
        ),
    )?;
    let heading_only = support::validator(&plugin_root, "--check")?;
    assert!(!heading_only.status.success());
    assert!(
        support::stderr(&heading_only)
            .contains("child external-gate wait must retain active goal and plan")
    );

    for negated_clause in [
        "A child external-gate wait MUST retain active goal and plan is not required.",
        "It is not required that child external-gate wait MUST retain active goal and plan.",
    ] {
        fs::write(
            &path,
            original.replace(
                "child external-gate wait MUST retain active goal and plan",
                negated_clause,
            ),
        )?;
        let negated_required = support::validator(&plugin_root, "--check")?;
        assert!(!negated_required.status.success(), "{negated_clause}");
    }
    Ok(())
}

#[test]
fn validator_rejects_required_ledger_phrases_that_appear_only_in_headings() -> TestResult {
    let (_temp, plugin_root) = support::copy_plugin_fixture()?;
    let path = plugin_root.join("skills/codex-orchestration/SKILL.md");
    let original = fs::read_to_string(&path)?;
    fs::write(
        &path,
        format!(
            "{}\n## Latest evidence\n",
            original.replace("latest evidence", "current proof")
        ),
    )?;

    let output = support::validator(&plugin_root, "--check")?;
    assert!(!output.status.success());
    assert!(support::stderr(&output).contains("latest evidence"));
    Ok(())
}

#[test]
fn validator_allows_compliant_negated_polling_prohibitions() -> TestResult {
    let (_temp, plugin_root) = support::copy_plugin_fixture()?;
    let path = plugin_root.join("skills/codex-orchestration/SKILL.md");
    fs::write(
        &path,
        format!(
            "{}\nMUST NOT keep polling and keep the goal active.\n",
            fs::read_to_string(&path)?
        ),
    )?;

    let output = support::validator(&plugin_root, "--check")?;
    assert!(output.status.success(), "{}", support::stderr(&output));
    Ok(())
}

#[test]
fn validator_rejects_inline_code_delimited_forbidden_policy() -> TestResult {
    let (_temp, plugin_root) = support::copy_plugin_fixture()?;
    let path = plugin_root.join("skills/codex-orchestration/SKILL.md");
    let original = fs::read_to_string(&path)?;
    fs::write(
        &path,
        format!(
            "{original}\nMUST call `update_goal`(status=\"blocked\") after a Sentinel BLOCK.\n"
        ),
    )?;

    let output = support::validator(&plugin_root, "--check")?;
    assert!(!output.status.success());
    assert!(support::stderr(&output).contains("must not block a usable owner"));
    Ok(())
}
