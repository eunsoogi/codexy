use super::*;

#[test]
fn original_table_reaches_native_bridge_with_bound_authority() -> TestResult {
    let original = "Terminal parent handoff:\n| Field | Value |\n| --- | --- |\n| event id | parent-handoff\\|parent-handoff\\|parent-679 |\n| issue/pr | 679 / PR not-created |\n| child task | child-679 |\n| parent task | parent-679 |\n| branch | branch |\n| worktree | worktree |\n| head | head |\n| clean/index | clean |\n| last proof | source validation |\n| current gate | review |\n| preserved reservation/artifacts | not-applicable |\n| parent next action | inspect |\n| delivery | confirmed |\n| task surface | codex task/thread |\n";
    let source = volatile(
        "parent-handoff",
        "parent-handoff",
        "parent-679",
        "parent-679",
        "child-679",
        false,
    );
    let context = LegacyContext {
        stable: stable(),
        owner: source.owner_worktree.owner,
        branch: source.owner_worktree.branch,
        worktree: source.owner_worktree.worktree,
        base: source.base_head_sha.base,
        dirty_index_state: source.dirty_index_state,
        checks: source.checks,
        unresolved_review_threads: source.unresolved_review_threads,
        selected_reviewer_state: source.selected_reviewer_state,
        verification: source.verification,
        active_obligation: source.active_obligation,
        external_gate: source.external_gate,
        child_task: source.child_task,
        parent_task: source.parent_task,
        preserved_artifacts: source.preserved_artifacts,
        authoritative_refresh_handles: source.authoritative_refresh_handles,
        delivery: source.delivery,
        task_surface: source.task_surface,
        omissions: source.omissions,
    };
    let envelope = migrate_legacy_handoff(original, &context)?;
    let fixture = CapsuleFixture::new("parent-handoff")?;
    let mut capsule: Value = serde_json::from_slice(&std::fs::read(&fixture.capsule)?)?;
    capsule["envelope"] = json!(envelope);
    std::fs::write(&fixture.capsule, serde_json::to_vec(&capsule)?)?;
    let accepted = fixture.command(bridge()).output()?;
    assert!(
        accepted.status.success(),
        "{}",
        String::from_utf8_lossy(&accepted.stderr)
    );
    assert!(
        !fixture.command(bridge()).status()?.success(),
        "replayed original event must fail"
    );
    assert!(
        migrate_legacy_handoff(
            &original.replace("| parent task | parent-679 |", "| parent task | other |"),
            &context
        )
        .is_err()
    );
    let unauthorized = original.replace(
        "| delivery | confirmed |",
        "| delivery | confirmed |\n| message permission | absent |",
    );
    assert!(migrate_legacy_handoff(&unauthorized, &context).is_err());
    Ok(())
}
