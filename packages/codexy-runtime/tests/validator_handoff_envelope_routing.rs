use crate::support::TestResult;
use codexy_runtime::validation::{
    BaseHeadSha, DirtyIndexState, HandoffAuthority, HandoffEnvelope, HandoffEvent, HandoffVolatile,
    IssuePrIdentity, OmissionReason, OwnerWorktree, ReviewThread, StableClassification,
    StableHandoff, StructuredClassification, validate_handoff,
};
use serde_json::json;
use std::collections::BTreeMap;

#[test]
fn stable_handoff_accepts_structured_task_surface_risk_classification() -> TestResult {
    let value = json!({
        "policy_digest": "",
        "workflow_profile": "strict",
        "task_classification": {
            "workflow": "review response",
            "surfaces": ["GitHub"],
            "risks": []
        },
        "selected_references": [
            "workflow_profiles",
            "task_classification",
            "review_profiles",
            "review_lifecycle",
            "proof_completion",
            "public_extension_contracts"
        ]
    });
    let stable: StableHandoff = serde_json::from_value(value.clone())?;
    assert_eq!(
        serde_json::to_value(stable)?["task_classification"],
        value["task_classification"]
    );
    Ok(())
}

#[test]
fn stable_handoff_validates_task_surface_union_and_fallback_risk_route() -> TestResult {
    let ordinary = ordinary_stable();
    let ordinary_json =
        HandoffEnvelope::new(ordinary.clone(), volatile("ordinary")).canonical_json()?;
    let ordinary_result = validate_handoff(&ordinary_json, &authority(ordinary));
    assert!(ordinary_result.is_ok(), "{ordinary_result:?}");

    let risky = risky_stable();
    let risky_json = HandoffEnvelope::new(risky.clone(), volatile("risky")).canonical_json()?;
    assert!(validate_handoff(&risky_json, &authority(risky.clone())).is_ok());

    let mut weakened = risky;
    weakened.selected_references = vec![
        "workflow_profiles".into(),
        "task_classification".into(),
        "review_profiles".into(),
        "review_lifecycle".into(),
        "proof_completion".into(),
        "public_extension_contracts".into(),
    ];
    let weakened_json =
        HandoffEnvelope::new(weakened.clone(), volatile("weakened")).canonical_json()?;
    assert!(validate_handoff(&weakened_json, &authority(weakened)).is_err());
    Ok(())
}

#[test]
fn structured_classification_rejects_unknown_fields() -> TestResult {
    let mut value = json!({
        "policy_digest": "",
        "workflow_profile": "strict",
        "task_classification": {
            "workflow": "review response",
            "surfaces": ["GitHub"],
            "risks": []
        },
        "selected_references": []
    });
    value["task_classification"]["unexpected"] = json!(true);
    assert!(serde_json::from_value::<StableHandoff>(value).is_err());
    Ok(())
}

fn ordinary_stable() -> StableHandoff {
    StableHandoff {
        policy_digest: String::new(),
        workflow_profile: "standard".into(),
        task_classification: StableClassification::Structured(StructuredClassification {
            workflow: "review response".into(),
            surfaces: vec!["GitHub".into(), "read-only/local".into()],
            risks: vec![],
        }),
        selected_references: vec![
            "workflow_profiles".into(),
            "task_classification".into(),
            "tdd_classification_policy".into(),
            "review_profiles".into(),
            "review_lifecycle".into(),
            "proof_completion".into(),
            "public_extension_contracts".into(),
        ],
    }
}

fn risky_stable() -> StableHandoff {
    StableHandoff {
        policy_digest: String::new(),
        workflow_profile: "strict".into(),
        task_classification: StableClassification::Structured(StructuredClassification {
            workflow: "review response".into(),
            surfaces: vec!["GitHub".into()],
            risks: vec!["destructive".into()],
        }),
        selected_references: vec![
            "workflow_profiles".into(),
            "task_classification".into(),
            "child_routing".into(),
            "proof_completion".into(),
        ],
    }
}

fn volatile(delta: &str) -> HandoffVolatile {
    HandoffVolatile {
        issue_pr_identity: IssuePrIdentity {
            issue: Some(663),
            pr: None,
        },
        owner_worktree: OwnerWorktree {
            owner: "child-owned".into(),
            branch: "eunsoogi/663-project-neutral-core".into(),
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
        checks: vec!["focused".into()],
        unresolved_review_threads: vec![ReviewThread {
            id: "none".into(),
            outdated: true,
        }],
        selected_reviewer_state: "exhausted".into(),
        verification: vec!["red-green".into()],
        active_obligation: "repair".into(),
        external_gate: "none".into(),
        next_action: "handoff".into(),
        child_task: Some("663".into()),
        parent_task: Some("parent".into()),
        preserved_artifacts: Some("fixtures".into()),
        delivery: "confirmed".into(),
        task_surface: "codex task/thread".into(),
        event: HandoffEvent {
            id: format!("delta|implementation|{delta}"),
            kind: "delta".into(),
            lane: "implementation".into(),
            subject: delta.into(),
            delta: "proof".into(),
        },
        authoritative_refresh_handles: vec!["git".into()],
        omissions: BTreeMap::from([(String::from("pr"), OmissionReason::NotCreated)]),
    }
}

fn authority(stable: StableHandoff) -> HandoffAuthority {
    HandoffAuthority::new(
        "head",
        "child-owned",
        "worktree",
        IssuePrIdentity {
            issue: Some(663),
            pr: None,
        },
        "eunsoogi/663-project-neutral-core",
        "base",
    )
    .with_stable(stable)
}
