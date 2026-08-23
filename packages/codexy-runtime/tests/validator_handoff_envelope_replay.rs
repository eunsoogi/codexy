use crate::support::TestResult;
use codexy_runtime::validation::*;
use std::collections::BTreeMap;

#[test]
fn failed_duplicate_batch_does_not_consume_event_id() -> TestResult {
    let text = HandoffEnvelope::new(stable(), volatile("603")).canonical_json()?;
    let authority = authority();
    assert!(validate_handoff_batch(&[&text, &text], &authority).is_err());
    assert!(validate_handoff(&text, &authority).is_ok());
    Ok(())
}

#[test]
fn successful_batch_publishes_to_shared_clones() -> TestResult {
    let first = HandoffEnvelope::new(stable(), volatile("603-first")).canonical_json()?;
    let second = HandoffEnvelope::new(stable(), volatile("603-second")).canonical_json()?;
    let authority = authority();
    let clone = authority.clone();
    assert!(validate_handoff_batch(&[&first, &second], &authority).is_ok());
    assert!(validate_handoff(&first, &clone).is_err());
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

fn volatile(id: &str) -> HandoffVolatile {
    HandoffVolatile {
        issue_pr_identity: IssuePrIdentity {
            issue: Some(603),
            pr: None,
        },
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
        omissions: BTreeMap::from([(String::from("pr"), OmissionReason::NotCreated)]),
        event: HandoffEvent {
            id: format!("new-head|implementation|{id}"),
            kind: "new-head".into(),
            lane: "implementation".into(),
            subject: id.into(),
            delta: "delta".into(),
        },
    }
}

fn authority() -> HandoffAuthority {
    HandoffAuthority::new(
        "head",
        "child-owned",
        "worktree",
        IssuePrIdentity {
            issue: Some(603),
            pr: None,
        },
        "eunsoogi/603-canonical-handoff-envelopes",
        "base",
    )
    .with_stable(stable())
}
