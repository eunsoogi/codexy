use super::*;

#[test]
fn readable_handoff_migrates_and_retains_authority_and_replay_checks() -> TestResult {
    let table = "Terminal parent handoff:\n| Field | Value |\n| --- | --- |\n| event id | terminal\\|implementation\\|603 |\n| issue/pr | 603 / PR not-created |\n| child task | child-603 |\n| parent task | parent-603 |\n| branch | eunsoogi/603-canonical-handoff-envelopes |\n| worktree | worktree |\n| head | head |\n| clean/index | clean |\n| last proof | focused RED |\n| current gate | none |\n| preserved reservation/artifacts | worktree reserved |\n| parent next action | review |\n| delivery | confirmed |\n| task surface | codex task/thread |\n";
    let table = table
        .replace("| event id |", "| Event id |")
        .replace("Terminal parent handoff:", "Parent handoff:");
    let migrated = migrate_legacy_handoff(&table, &migration_context())?;
    let auth = authority();
    assert!(validate_handoff(&migrated, &auth).is_ok());
    assert!(validate_handoff(&migrated, &auth).is_err());
    assert!(
        validate_handoff(
            &migrated,
            &authority_with("stale", "child-owned", Some(603))
        )
        .is_err()
    );
    assert!(
        migrate_legacy_handoff(
            &table.replace("| head | head |", "| head | head |\n| head | stale |"),
            &migration_context()
        )
        .is_err()
    );
    let historical = table
        .lines()
        .map(|line| format!("> {line}\n"))
        .collect::<String>();
    assert!(migrate_legacy_handoff(&historical, &migration_context()).is_err());
    assert!(
        migrate_legacy_handoff(
            &table.replace(
                "| parent task | parent-603 |",
                "| parent task | other-parent |"
            ),
            &migration_context()
        )
        .is_err()
    );
    Ok(())
}

#[test]
fn readable_delta_preserves_lane_and_event_authority() -> TestResult {
    let text = "## Lane\n| Field | Value |\n| --- | --- |\n| issue | 603 |\n| PR | not-created |\n| branch | eunsoogi/603-canonical-handoff-envelopes |\n| owner | child-owned |\n| worktree | worktree |\n| head SHA | head |\n| base SHA | base |\n\n## Delta\n| Field | Value |\n| --- | --- |\n| event id | new-head\\|implementation\\|603 |\n| event kind | new-head |\n| delta | focused checks passed |\n\n## Next\n| Field | Value |\n| --- | --- |\n| one next action | review |\n";
    let wrapped = text
        .replace("focused checks passed |", "focused checks\npassed |")
        .replace("| owner | child-owned |", "| owner | `child-owned` |");
    let migrated = migrate_legacy_handoff(&wrapped, &migration_context())?;
    assert!(validate_handoff(&migrated, &authority()).is_ok());
    assert!(
        migrate_legacy_handoff(
            &text.replace("| owner | child-owned |", "| owner | parent-owned |"),
            &migration_context()
        )
        .is_err()
    );
    Ok(())
}

pub(super) fn migration_context() -> LegacyContext {
    let source = volatile("context");
    LegacyContext {
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
        omissions: omissions(false),
    }
}
