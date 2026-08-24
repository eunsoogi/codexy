use crate::support::TestResult;
use codexy_runtime::validation::*;
use serde_json::{Value, json};
use std::{collections::BTreeMap, process::Command};

#[test]
fn native_bridge_binds_each_direction_and_rejects_pairwise_relabeling() -> TestResult {
    let bridge = bridge();
    for consumer in ["compaction", "fresh-child", "parent-handoff"] {
        let fixture = CapsuleFixture::new(consumer)?;
        let output = fixture.command(bridge).output()?;
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let result: Value = serde_json::from_slice(&output.stdout)?;
        assert_eq!(result["consumer"], consumer);
    }
    for (original, relabeled) in [
        ("compaction", "fresh-child"),
        ("fresh-child", "parent-handoff"),
        ("parent-handoff", "compaction"),
    ] {
        let fixture = CapsuleFixture::relabeled(original, relabeled)?;
        assert!(!fixture.command(bridge).status()?.success());
    }
    Ok(())
}

#[test]
fn native_bridge_rejects_subject_role_conflicts_and_duplicate_replay() -> TestResult {
    let bridge = bridge();
    let fixture = CapsuleFixture::new("fresh-child")?;
    assert!(fixture.command(bridge).status()?.success());
    assert!(!fixture.command(bridge).status()?.success());
    let fixture = CapsuleFixture::subject_conflict()?;
    assert!(!fixture.command(bridge).status()?.success());
    let fixture = CapsuleFixture::authority_conflict()?;
    assert!(!fixture.command(bridge).status()?.success());
    Ok(())
}

#[test]
fn native_bridge_serializes_concurrent_replay_updates() -> TestResult {
    let bridge = bridge();
    let fixture = CapsuleFixture::new("fresh-child")?;
    let children = (0..20)
        .map(|_| fixture.command(bridge).spawn())
        .collect::<Result<Vec<_>, _>>()?;
    let successes = children
        .into_iter()
        .map(|mut child| child.wait())
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .filter(std::process::ExitStatus::success)
        .count();
    assert_eq!(successes, 1, "exactly one concurrent replay may commit");
    Ok(())
}

fn bridge() -> &'static str {
    let bridge = option_env!("CARGO_BIN_EXE_codexy-handoff-validate");
    assert!(bridge.is_some(), "missing native handoff bridge binary");
    bridge.unwrap_or_default()
}

struct CapsuleFixture {
    _temporary: tempfile::TempDir,
    capsule: std::path::PathBuf,
    authority: std::path::PathBuf,
}

impl CapsuleFixture {
    fn new(consumer: &str) -> TestResult<Self> {
        Self::build(consumer, consumer, false, false)
    }

    fn relabeled(original: &str, relabeled: &str) -> TestResult<Self> {
        Self::build(original, relabeled, false, false)
    }

    fn subject_conflict() -> TestResult<Self> {
        Self::build("fresh-child", "fresh-child", true, false)
    }

    fn authority_conflict() -> TestResult<Self> {
        Self::build("fresh-child", "fresh-child", false, true)
    }

    fn build(
        original: &str,
        consumer: &str,
        subject_conflict: bool,
        authority_conflict: bool,
    ) -> TestResult<Self> {
        let temporary = tempfile::tempdir()?;
        let parent = "parent-679";
        let child = "child-679";
        let (kind, lane, subject, source, target) = match original {
            "compaction" => ("compaction-resume", "compaction", child, parent, child),
            "fresh-child" => (
                "fresh-child-continuation",
                "fresh-child",
                child,
                parent,
                child,
            ),
            "parent-handoff" => ("parent-handoff", "parent-handoff", parent, child, parent),
            _ => unreachable!(),
        };
        let event_subject = if subject_conflict {
            "other-task"
        } else {
            subject
        };
        let volatile = volatile(kind, lane, event_subject, parent, child, authority_conflict);
        let envelope = HandoffEnvelope::new(stable(), volatile).canonical_json()?;
        let capsule = temporary.path().join("capsule.json");
        let authority = temporary.path().join("authority.json");
        std::fs::write(
            &authority,
            serde_json::to_vec(&json!({
                "schema": "codexy.handoff-authority.v1",
                "currentHead": "head", "owner": "child-owned", "worktree": "worktree",
                "issue": 679, "pr": null, "branch": "branch", "base": "base", "stable": stable(),
            }))?,
        )?;
        std::fs::write(
            &capsule,
            serde_json::to_vec(&json!({
                "schema": "codexy.resumable-context-capsule.v1",
                "consumer": consumer,
                "subject": subject,
                "sourceTask": source,
                "targetTask": target,
                "replayPath": temporary.path().join("replay.json"),
                "envelope": envelope,
            }))?,
        )?;
        Ok(Self {
            _temporary: temporary,
            capsule,
            authority,
        })
    }

    fn command(&self, bridge: &str) -> Command {
        let mut command = Command::new(bridge);
        command
            .arg("--capsule")
            .arg(&self.capsule)
            .arg("--authority")
            .arg(&self.authority);
        command
    }
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

fn volatile(
    kind: &str,
    lane: &str,
    subject: &str,
    parent: &str,
    child: &str,
    conflict: bool,
) -> HandoffVolatile {
    let (owner, branch, head) = if conflict {
        ("forged-owner", "forged-branch", "forged-head")
    } else {
        ("child-owned", "branch", "head")
    };
    HandoffVolatile {
        issue_pr_identity: IssuePrIdentity {
            issue: Some(679),
            pr: None,
        },
        owner_worktree: OwnerWorktree {
            owner: owner.into(),
            branch: branch.into(),
            worktree: "worktree".into(),
        },
        base_head_sha: BaseHeadSha {
            base: "base".into(),
            head: head.into(),
        },
        dirty_index_state: DirtyIndexState {
            dirty: false,
            index: false,
        },
        checks: vec!["focused".into()],
        unresolved_review_threads: vec![],
        selected_reviewer_state: "pending".into(),
        verification: vec!["installed".into()],
        active_obligation: "validate".into(),
        external_gate: "none".into(),
        next_action: "continue".into(),
        child_task: Some(child.into()),
        parent_task: Some(parent.into()),
        preserved_artifacts: None,
        delivery: "confirmed".into(),
        task_surface: "codex-task".into(),
        event: HandoffEvent {
            id: format!("{kind}|{lane}|{subject}"),
            kind: kind.into(),
            lane: lane.into(),
            subject: subject.into(),
            delta: "capsule".into(),
        },
        authoritative_refresh_handles: vec![],
        omissions: BTreeMap::from([
            ("pr".into(), OmissionReason::NotCreated),
            ("preserved_artifacts".into(), OmissionReason::NotApplicable),
            (
                "authoritative_refresh_handles".into(),
                OmissionReason::NotApplicable,
            ),
        ]),
    }
}
