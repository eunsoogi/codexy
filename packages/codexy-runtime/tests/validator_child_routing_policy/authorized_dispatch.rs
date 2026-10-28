//! #1424 source-contract regressions. These checks do not prove app permission,
//! task creation, concurrent execution, or integration conflict resolution.

use super::{LUNA, TERRA, check, fallback, hard_work_cases, request, selected};
use crate::support::TestResult;
use serde_json::json;

#[test]
fn routing_needs_no_extra_approval_or_path_allocation_fields() -> TestResult {
    // The resolver chooses recipients; actual authority, capacity and worktrees
    // belong to the host/supervisor, so no duplicate approval oracle is added.
    for pair in [LUNA, TERRA] {
        let classification = if pair == LUNA { "simple" } else { "general" };
        let mut creation = request(classification, pair);
        // Terra requires the same explicit hard-work evidence after main integration;
        // dispatch authority must not weaken the recipient selection contract.
        if pair == TERRA {
            creation["hard_work"] = hard_work_cases()?.remove(0);
        }
        check(creation.clone(), selected("generic", "create_thread", pair))?;
        let mut followup = creation.clone();
        followup["schema"] = json!("codexy.child-routing-request.v2");
        followup["codex_thread_operation"] = json!("send_message_to_thread");
        followup["codex_thread_direction"] = json!("parent_to_generic");
        followup["recipient_assignment"] = json!({"model":pair.0,"thinking":pair.1});
        check(
            followup,
            selected("parent_to_generic", "send_message_to_thread", pair),
        )?;
        // Scoped dispatch authority cannot manufacture unavailable capabilities.
        let mut unavailable = creation;
        unavailable["codex_thread_capabilities"] = json!(null);
        check(unavailable, fallback())?;
    }
    Ok(())
}

#[test]
fn canonical_guidance_preserves_scoped_authority_and_parallel_dispatch() -> TestResult {
    let root = codexy_runtime::paths::repository_root().join("plugins/codexy/skills");
    // Guard the contradictory source clauses seen in #1424, while keeping live
    // app evidence a separate parent-owned acceptance gate.
    for (path, required, obsolete) in [
        (
            "orchestration/SKILL.md",
            "MUST NOT scan or compare paths as a dispatch gate",
            "Shared-file or resource conflicts",
        ),
        (
            "orchestration/SKILL.md",
            "this authority supersedes the task-creation exclusions in `classification-and-control.md`",
            "",
        ),
        (
            "orchestration/references/child-routing-policy.md",
            "MUST NOT ask for approval again per child or per message",
            "MUST preserve current task-creation/message authority",
        ),
        (
            "orchestration/references/thread-and-worktree-routing.md",
            "even when owned paths overlap",
            "disjoint lanes MAY run concurrently",
        ),
        (
            "orchestration/references/parent-supervision.md",
            "without scanning paths or gating on path overlap",
            "",
        ),
        (
            "orchestration/references/goal-transition-reporting.md",
            "bounded reports and recovery messages without another approval",
            "",
        ),
        (
            "goal-lifecycle/SKILL.md",
            "MUST NOT be mistaken for host goal-tool permission",
            "",
        ),
    ] {
        let content = std::fs::read_to_string(root.join(path))?;
        let prose = content.split_whitespace().collect::<Vec<_>>().join(" ");
        assert!(
            prose.contains(required),
            "{path}: missing #1424 authority/dispatch contract"
        );
        assert!(
            obsolete.is_empty() || !prose.contains(obsolete),
            "{path}: stale dispatch gate"
        );
    }
    let worktree = std::fs::read_to_string(
        root.join("orchestration/references/thread-and-worktree-routing.md"),
    )?;
    let prose = worktree.split_whitespace().collect::<Vec<_>>().join(" ");
    for invariant in [
        "one owner and isolated app-managed worktree per issue",
        "Real integration conflicts MUST be resolved before merge through the owning task's branch",
        "MUST NOT infer parent permission inheritance",
        "MUST NOT scan or compare paths as a dispatch gate",
    ] {
        assert!(
            prose.contains(invariant),
            "missing preserved boundary: {invariant}"
        );
    }
    Ok(())
}
