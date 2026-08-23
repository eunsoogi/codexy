use crate::support::TestResult;
use codexy_runtime::validation::*;
use serde_json::{Value, json};
use std::collections::BTreeMap;
#[test]
fn canonical_serialization_separates_stable_and_volatile_payloads() -> TestResult {
    let first = HandoffEnvelope::new(stable(), volatile("red"));
    let canonical = first.canonical_json()?;
    let full = serde_json::to_string(&json!({
        "schema": &first.schema,
        "stable": &first.stable,
        "volatile": &first.volatile,
        "stable_identity": &first.stable_identity,
        "volatile_identity": &first.volatile_identity,
    }))?;
    let mut reordered: Value = serde_json::from_str(&canonical)?;
    let volatile_payload = reordered["volatile"].clone();
    reordered = json!({
        "volatile_identity": reordered["volatile_identity"],
        "volatile": volatile_payload,
        "schema": reordered["schema"],
        "stable_identity": reordered["stable_identity"],
    });
    assert_eq!(
        canonicalize_handoff(&serde_json::to_string(&reordered)?)?,
        canonical
    );
    assert_eq!(canonicalize_handoff(&canonical)?, canonical);
    assert!(validate_handoff(&full, &authority()).is_err());
    let changed = HandoffEnvelope::new(stable(), volatile("green"));
    assert_eq!(
        serde_json::to_vec(&first.stable)?,
        serde_json::to_vec(&changed.stable)?
    );
    assert!(canonical.len() < full.len());
    assert!(canonical.as_bytes().chunks(4).count() < full.as_bytes().chunks(4).count());
    Ok(())
}
#[test]
fn validation_rejects_conflicts_stale_heads_unknown_fields_cross_owner_and_duplicates() -> TestResult
{
    let canonical = HandoffEnvelope::new(stable(), volatile("red")).canonical_json()?;
    assert!(validate_handoff(&canonical, &authority()).is_ok());
    let mut unknown: Value = serde_json::from_str(&canonical)?;
    unknown["unknown"] = json!(true);
    rejects(&serde_json::to_string(&unknown)?, authority());
    let mut conflict: Value = serde_json::from_str(&canonical)?;
    conflict["stable_identity"] = json!("codexy.handoff.stable.v1:conflict");
    rejects(&serde_json::to_string(&conflict)?, authority());
    for (head, owner) in [("stale", "child-owned"), ("head", "parent-owned")] {
        rejects(&canonical, authority_with(head, owner, Some(603)));
    }
    assert!(validate_handoff_batch(&[&canonical, &canonical], &authority()).is_err());
    let batch_authority = authority();
    assert!(validate_handoff_batch(&[&canonical], &batch_authority).is_ok());
    assert!(validate_handoff(&canonical, &batch_authority).is_err());
    let cloned_authority = authority();
    let original_authority = cloned_authority.clone();
    assert!(validate_handoff(&canonical, &cloned_authority).is_ok());
    assert!(validate_handoff(&canonical, &original_authority).is_err());
    let mut wrong_branch = volatile("branch");
    wrong_branch.owner_worktree.branch = "other-branch".into();
    rejects_envelope(wrong_branch, authority())?;
    let mut wrong_base = volatile("base");
    wrong_base.base_head_sha.base = "other-base".into();
    rejects_envelope(wrong_base, authority())?;
    let mut wrong_issue = volatile("issue");
    wrong_issue.issue_pr_identity.issue = Some(604);
    rejects_envelope(wrong_issue, authority())?;
    Ok(())
}
#[test]
fn bounded_migration_accepts_one_legacy_boundary_and_rejects_mixed_input() -> TestResult {
    let legacy = "## Lane\n\n- issue: 603\n- PR: not-created\n- branch: eunsoogi/603-canonical-handoff-envelopes\n- owner: child-owned\n- worktree: worktree\n- head SHA: head\n- base SHA: base\n\n## Delta\n\n- event id: new-head|implementation|603\n- event kind: new-head\n- delta: RED captured\n\n## Next\n\n- one next action: implement\n";
    let context = migration_context();
    let migrated = migrate_legacy_handoff(legacy, &context)?;
    assert!(validate_handoff(&migrated, &authority()).is_ok());
    let terminal = "Terminal parent handoff: event id=terminal|implementation|603;\nissue/pr=603 / PR not-created;\nchild task=child-603;\nparent task=parent-603;\nbranch=eunsoogi/603-canonical-handoff-envelopes;\nworktree=worktree;\nhead=head;\nclean/index=clean;\nlast proof=focused RED;\ncurrent gate=none;\npreserved reservation/artifacts=worktree reserved;\nparent next action=review;\ndelivery=confirmed;\ntask surface=codex task/thread";
    let mut terminal_context = context.clone();
    terminal_context.delivery = "context-delivery".into();
    terminal_context.task_surface = "context-surface".into();
    let migrated_terminal = migrate_legacy_handoff(terminal, &terminal_context)?;
    assert!(validate_handoff(&migrated_terminal, &authority()).is_ok());
    let migrated_value: Value = serde_json::from_str(&migrated_terminal)?;
    for (field, expected) in [
        ("child_task", "child-603"),
        ("parent_task", "parent-603"),
        ("preserved_artifacts", "worktree reserved"),
        ("delivery", "confirmed"),
        ("task_surface", "codex task/thread"),
    ] {
        assert_eq!(migrated_value["volatile"][field], expected);
    }
    assert_eq!(
        migrated_value["volatile"]["authoritative_refresh_handles"][0],
        "refresh-603"
    );
    assert!(
        migrate_legacy_handoff(&format!("{legacy}\n{terminal}\n{terminal}"), &context).is_err()
    );
    assert!(migrate_legacy_handoff(&format!("{legacy}\n{terminal}"), &context).is_err());
    assert!(migrate_legacy_handoff("## Lane\n", &context).is_err());
    assert!(migrate_legacy_handoff(&format!("{terminal}; {terminal}"), &context).is_err());
    assert!(migrate_legacy_handoff(&"x".repeat(64 * 1024 + 1), &context).is_err());
    let mut invalid = volatile("omission");
    invalid
        .omissions
        .insert("issue".into(), OmissionReason::NotApplicable);
    rejects_envelope(invalid, authority())?;
    Ok(())
}
#[test]
fn general_codex_handoff_can_mark_issue_and_pr_not_applicable() -> TestResult {
    let envelope = HandoffEnvelope::new(stable(), volatile_without_issue());
    let canonical = envelope.canonical_json()?;
    assert!(validate_handoff(&canonical, &authority_with("head", "child-owned", None)).is_ok());
    let value: Value = serde_json::from_str(&canonical)?;
    for field in ["issue", "authoritative_refresh_handles"] {
        assert_eq!(value["volatile"]["omissions"][field], "not_applicable");
    }
    Ok(())
}
fn stable() -> StableHandoff {
    StableHandoff {
        policy_digest: String::new(),
        workflow_profile: "strict".into(),
        task_classification: "implementation".into(),
        selected_references: vec![
            "workflow_profiles".into(),
            "task_classification".into(),
            "tdd_classification_policy".into(),
            "execution_budget".into(),
            "proof_completion".into(),
        ],
    }
}
fn volatile(delta: &str) -> HandoffVolatile {
    HandoffVolatile {
        issue_pr_identity: identity(Some(603)),
        owner_worktree: OwnerWorktree {
            owner: "child-owned".into(),
            branch: "eunsoogi/603-canonical-handoff-envelopes".into(),
            worktree: "worktree".into(),
        },
        base_head_sha: BaseHeadSha {
            base: "base".into(),
            head: "head".into(),
        },
        dirty_index_state: DirtyIndexState {
            dirty: false,
            index: false,
        },
        checks: vec!["not_created".into()],
        unresolved_review_threads: vec![ReviewThread {
            id: "none".into(),
            outdated: true,
        }],
        selected_reviewer_state: "pending".into(),
        verification: vec!["focused".into()],
        active_obligation: "implement envelope".into(),
        external_gate: "none".into(),
        next_action: "implement".into(),
        child_task: Some("child-603".into()),
        parent_task: Some("parent-603".into()),
        preserved_artifacts: Some("worktree reserved".into()),
        authoritative_refresh_handles: vec!["refresh-603".into()],
        delivery: "confirmed".into(),
        task_surface: "codex task/thread".into(),
        omissions: omissions(false),
        event: HandoffEvent {
            id: "new-head|implementation|603".into(),
            kind: "new-head".into(),
            lane: "implementation".into(),
            subject: "603".into(),
            delta: delta.into(),
        },
    }
}
fn volatile_without_issue() -> HandoffVolatile {
    let mut value = volatile("general");
    value.issue_pr_identity = identity(None);
    value.authoritative_refresh_handles.clear();
    value.omissions = omissions(true);
    value
}
fn omissions(general: bool) -> BTreeMap<String, OmissionReason> {
    let mut result = BTreeMap::from([(String::from("pr"), OmissionReason::NotCreated)]);
    if general {
        result.extend(BTreeMap::from([
            (String::from("issue"), OmissionReason::NotApplicable),
            (
                String::from("authoritative_refresh_handles"),
                OmissionReason::NotApplicable,
            ),
            (String::from("pr"), OmissionReason::NotApplicable),
        ]));
    }
    result
}
fn authority() -> HandoffAuthority {
    authority_with("head", "child-owned", Some(603))
}
fn authority_with(head: &str, owner: &str, issue: Option<u64>) -> HandoffAuthority {
    HandoffAuthority::new(
        head,
        owner,
        "worktree",
        identity(issue),
        "eunsoogi/603-canonical-handoff-envelopes",
        "base",
    )
    .with_stable(stable())
}
fn identity(issue: Option<u64>) -> IssuePrIdentity {
    IssuePrIdentity { issue, pr: None }
}
fn rejects(text: &str, authority: HandoffAuthority) {
    assert!(validate_handoff(text, &authority).is_err());
}
fn rejects_envelope(value: HandoffVolatile, authority: HandoffAuthority) -> TestResult {
    rejects(
        &HandoffEnvelope::new(stable(), value).canonical_json()?,
        authority,
    );
    Ok(())
}
fn migration_context() -> LegacyContext {
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
