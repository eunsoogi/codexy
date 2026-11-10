//! Builds the minimal admitted event for each topology concern, using the
//! primary checkout as the child-thread creation CWD.

use super::{Concern, TestResult};
use serde_json::{Value, json};

pub(super) fn admitted_payload(concern: &Concern, event: &str) -> TestResult<Value> {
    let tool_input = match concern.id {
        "thread-delivery" => json!({
            "threadId":"test-worker",
            "model":"gpt-6-luna",
            "thinking":"max"
        }),
        "child-thread-creation" => json!({
            "model":"gpt-6-luna",
            "thinking":"max",
            "target":{
                "type":"project",
                "projectId":"local-test-project",
                "environment":{"type":"worktree"}
            }
        }),
        // Valid native spawn fixtures request the required no-history setting.
        "subagent-ownership" => json!({"agent_type":"explorer","message":"Bounded read-only inspection.","fork_turns":"none"}),
        _ => unreachable!(),
    };
    let cwd = if concern.id == "child-thread-creation" {
        let root = codexy_runtime::paths::repository_root();
        let output = std::process::Command::new("git")
            .arg("-C")
            .arg(root)
            .args(["worktree", "list", "--porcelain"])
            .output()?;
        if !output.status.success() {
            return Err(std::io::Error::other("git worktree list failed").into());
        }
        String::from_utf8(output.stdout)?
            .lines()
            .find_map(|line| line.strip_prefix("worktree "))
            .ok_or("primary checkout")?
            .to_owned()
    } else {
        codexy_runtime::paths::repository_root()
            .display()
            .to_string()
    };
    let mut payload = json!({"hook_event_name": event, "tool_name": concern.tool, "tool_input": tool_input,
        "cwd": cwd});
    if concern.id == "thread-delivery" {
        // Valid delivery fixtures model the host route that admission now requires.
        payload["session_id"] = json!("test-orchestrator");
        payload["codexy_thread_delivery"] = json!({
            "schema":"codexy.thread-delivery.v2",
            "authenticated":true,
            "direction":"root_to_child",
            "sender_thread_id":"test-orchestrator",
            "target_thread_id":"test-worker",
            "target_model":"gpt-6-luna",
            "target_thinking":"max"
        });
    }
    Ok(payload)
}
