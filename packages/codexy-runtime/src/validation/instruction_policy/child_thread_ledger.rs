use std::path::Path;

use super::clauses::{reject_all, require_all};

pub(super) fn check(path: &Path, text: &str, errors: &mut Vec<String>) {
    if path.ends_with("skills/dreaming/SKILL.md") {
        require_all(
            path,
            text,
            errors,
            "dreaming skill must require a durable active/waiting child thread ledger",
            &[
                "in-progress/waiting child thread list",
                "issue/pr",
                "thread id",
                "status",
                "owner state",
                "blocker",
                "latest evidence",
                "next action",
                "removed from the ledger",
                "canonical worktree cwd",
                "frozen head",
                "clean/index state",
                "referencing specialist or sentinel",
                "explicit release/archive state",
                "must remain as worktree reservations",
                "must not recycle the worktree",
            ],
        );
    } else if path.ends_with("skills/orchestration/SKILL.md") {
        require_all(
            path,
            text,
            errors,
            "orchestration skill must maintain the active child thread ledger",
            &[
                "active child codex app threads must be capped at 5",
                "existing issue/pr owner thread",
                "reuse it when present",
                "completed child threads must remain reserved",
                "unavailable archive/delete surface as unresolved reservation evidence",
                "blocked/rate-limited child lanes",
                "latest evidence",
                "compaction recovery",
                "event-driven refresh",
                "packaged specialist subagents must not be counted",
                "canonical worktree cwd",
                "frozen head",
                "clean/index state",
                "explicit release/archive state",
                "must keep its reservation active",
                "must not silently recycle that worktree",
                "short-lived child implementation goal",
                "must not autonomously poll",
                "exactly one terminal unavailable report",
                "must not retry the parent message",
                "no full conversation transfer",
                "no full agent-tree listing",
                "parent or child must retain its active goal and plan during a nonterminal external-gate wait while an implementation obligation remains",
                "child must use one nonterminal wait handoff",
                "goal state=active",
                "goal transition=none",
                "must not call update_goal(complete) or update_goal(blocked) merely for that wait",
                "inspect archive candidates and the active reservation ledger",
                "may archive only terminal, unreferenced, clean and unreserved worktree lanes with no open pr or pending gate",
                "must not archive pr owners or dirty/reserved candidates",
                "record the decision in setup evidence",
                "usable existing owner must record the `block` and update the plan to a repair step",
                "add faithful red coverage, repair, rerun terminal proof, then invoke exactly one fresh sentinel review for the new file state or head",
                "material child event",
                "actionable review feedback",
                "route actionable review feedback",
                "replacement-owner availability",
                "start a replacement owner",
                "validate the stable event identity",
                "consume it in the same turn",
                "perform the authorized parent-owned next action",
                "record a concrete execution blocker",
                "acknowledgement-only output must not satisfy consumption",
                "duplicate stable event identities must remain deduplicated with no parent action",
                "unchanged continuation observations must not create assistant turns",
            ],
        );
        reject_all(
            path,
            text,
            errors,
            "orchestration skill must not allow specialist subagents to count against the child thread cap",
            &["packaged specialist subagents must not be counted unless"],
        );
        reject_all(
            path,
            text,
            errors,
            "orchestration skill must reject legacy external-gate goal retention and autonomous polling",
            &["must keep polling and keep the goal active"],
        );
        reject_all(
            path,
            text,
            errors,
            "orchestration skill must reject runtime-only goal retention after completion",
            &[
                "child external-gate wait must retain active goal and plan solely after every implementation obligation is complete",
            ],
        );
        reject_all(
            path,
            text,
            errors,
            "orchestration skill must not complete a goal for an external-gate wait",
            &[
                "only an external gate remains, the child must send exactly one terminal parent handoff, call `update_goal(complete)`",
                "child with only an external gate remaining must call update_goal(complete) before waiting",
            ],
        );
        reject_all(
            path,
            text,
            errors,
            "orchestration skill must retain an active goal during a nonterminal wait",
            &["child external-gate wait must end its active goal and plan before waiting"],
        );
        reject_all(
            path,
            text,
            errors,
            "orchestration skill must not block a usable owner goal after Sentinel BLOCK",
            &["must call update_goal(status=\"blocked\") after a sentinel block"],
        );
        reject_all(
            path,
            text,
            errors,
            "orchestration skill must not replace a usable owner after Sentinel BLOCK",
            &["must create a replacement thread after a sentinel block"],
        );
    } else if path.ends_with("skills/orchestration/references/thread-and-worktree-routing.md") {
        require_all(
            path,
            text,
            errors,
            "thread routing must require live worktree reservation preflight",
            &[
                "live worktree reservation preflight",
                "reservation map",
                "every active or waiting specialist or sentinel",
                "must not create or fork the new thread",
                "must fail setup before allocation",
                "host allocator blocker",
                "dirty or locked candidate worktrees",
            ],
        );
    } else if path.ends_with("skills/orchestration/references/goal-transition-reporting.md") {
        require_all(
            path,
            text,
            errors,
            "goal-transition reporting guidance must preserve the static parent receipt contract",
            &[
                "source_thread_id",
                "actual codex task/thread messaging surface",
                "agents.send_message('/root')",
                "stable transition key",
                "before `create_goal`",
                "after every goal tool call, including `get_goal`",
                "must not execute until parent delivery is confirmed",
                "exact tool result",
                "task cwd that differs from the canonical reserved worktree",
            ],
        );
    }
}
