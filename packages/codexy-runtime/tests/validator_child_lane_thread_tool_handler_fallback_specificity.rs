fn vague_fallback_evidence(route_evidence: &str) -> String {
    format!(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread; {route_evidence}; separate dogfood issue: #205.
Maintainer reassignment: none
"#
    )
}
fn tracking_issue_evidence(issue_evidence: &str) -> String {
    format!(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread; no fallback route was available; {issue_evidence}.
Maintainer reassignment: none
"#
    )
}

fn separate_metadata_evidence(fallback_field: &str, tracking_field: &str) -> String {
    format!(
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread.
{fallback_field}
{tracking_field}
Maintainer reassignment: none
"#
    )
}

fn preceding_metadata_evidence() -> String {
    r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Dogfooding/tool-exposure defect: recorded runtime missing-handler evidence for codex_app.read_thread.
Fallback route: no fallback route was available
Tracking issue: #205
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Maintainer reassignment: none
"#
    .to_owned()
}

fn preceding_metadata_without_defect_evidence() -> String {
    r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Fallback route: no fallback route was available
Tracking issue: #205
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Maintainer reassignment: none
"#
    .to_owned()
}

#[test]
fn validator_preserves_fallback_route_and_tracking_issue_matrix()
-> Result<(), Box<dyn std::error::Error>> {
    let mut cases: Vec<(String, String, bool)> = Vec::new();
    for route in [
        "fallback route used",
        "fallback routed",
        "fallback route: used",
        "fallback route: not used",
        "fallback route: not routed",
    ] {
        cases.push((route.to_owned(), vague_fallback_evidence(route), false));
    }
    for route in [
        "fallback route: it is false that no fallback route was available",
        "fallback route: no fallback route was available? no",
        "fallback route: no fallback route available: no",
        "fallback route: no fallback route was available: no",
        "fallback route: no fallback route available: false",
        "fallback route: no fallback route was available? false",
        "fallback route: no fallback route available = false",
        "fallback route: no fallback route was available = false",
        "fallback route: no fallback route available = no",
        "fallback route: no fallback route was available - false",
    ] {
        cases.push((route.to_owned(), vague_fallback_evidence(route), false));
    }
    cases.push((
        "reasoned no-route evidence".to_owned(),
        vague_fallback_evidence(
            "fallback route: no fallback route was available because the child thread was not available",
        ),
        true,
    ));
    for issue in [
        "separate dogfood issue: #205 tracks the missing-handler exposure",
        "tracking issue: missing-handler exposure #205",
        "tracking issue: #205 is not yet closed",
        "tracking issue: #205 covers handler not available",
    ] {
        cases.push((issue.to_owned(), tracking_issue_evidence(issue), true));
    }
    cases.push((
        "fallback and no-route metadata".to_owned(),
        vague_fallback_evidence(
            "fallback route: fallback route available? no; no fallback route was available",
        ),
        true,
    ));
    for issue in [
        "tracking issue: missing",
        "tracking issue: none, see #205",
        "tracking issue: missing (#205)",
        "tracking issue: no issue, see #205",
        "tracking issue: no issue (#205)",
        "tracking issue: no issue - #205",
        "tracking issue: no separate issue #205",
        "tracking issue: issue not created for #205",
        "tracking issue: issue not yet created for #205",
        "tracking issue: issue not yet filed for #205",
        "follow-up issue: issue not yet created for #205",
        "tracking issue: #205 not filed yet",
        "tracking issue: #205 not created",
        "tracking issue will be created as #205",
        "tracking issue to be filed as #205",
        "tracking issue: to be filed as #205",
        "follow-up issue pending #205",
        "follow-up issue pending: #205",
        "tracking issue pending: #205",
        "tracking issue will be created: #205",
        "- tracking issue will be created as #205",
        "- follow-up issue pending #205",
    ] {
        cases.push((issue.to_owned(), tracking_issue_evidence(issue), false));
    }
    for tracking in [
        "Tracking issue: #205",
        "Tracked in issue: #205",
        "Tracked by issue: #205",
    ] {
        cases.push((
            tracking.to_owned(),
            separate_metadata_evidence("Fallback route: no fallback route was available", tracking),
            true,
        ));
    }
    cases.push((
        "fallback route used metadata".to_owned(),
        separate_metadata_evidence(
            "Fallback route used: parent posted the handoff in the child thread",
            "Tracking issue: #205",
        ),
        true,
    ));
    cases.push((
        "bulleted handoff fields".to_owned(),
        r#"Owner decision: parent-owned for thread/worktree tool discovery only; child routing required
Tool search: discovered codex_app.read_thread as an available thread tool.
Invocation evidence: codex_app.read_thread failed with `No handler registered for tool: read_thread`.
Dogfooding/tool-exposure defect:
- recorded runtime missing-handler evidence for codex_app.read_thread
- Fallback route: no fallback route was available
- Tracking issue: #205
Maintainer reassignment: none
"#
            .to_owned(),
        true,
    ));
    cases.push((
        "preceding metadata".to_owned(),
        preceding_metadata_evidence(),
        true,
    ));
    cases.push((
        "preceding metadata without defect".to_owned(),
        preceding_metadata_without_defect_evidence(),
        false,
    ));
    cases.push((
        "GitHub issue URL".to_owned(),
        tracking_issue_evidence("tracking issue: https://github.com/eunsoogi/codexy/issues/205"),
        true,
    ));
    cases.push((
        "malformed GitHub issue URL suffix".to_owned(),
        tracking_issue_evidence("tracking issue: https://github.com/eunsoogi/codexy/issues/205abc"),
        false,
    ));

    for (name, evidence, expected_success) in cases {
        let output = crate::support::validator_child_lane_ownership(&evidence)?;
        assert_eq!(output.status.success(), expected_success, "{name}");
    }
    Ok(())
}
