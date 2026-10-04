use super::*;

#[test]
fn parent_worktree_is_rejected_before_worker_creation() -> TestResult {
    let (_temp, _primary, worktree) = git_worktree_fixture()?;
    for tool in TOOLS {
        for event in ["PreToolUse", "PermissionRequest"] {
            let mut input = pre_tool_input_at(
                tool,
                json!({"model":"gpt-6-luna","thinking":"max"}),
                &worktree,
            );
            input["hook_event_name"] = json!(event);
            let output = hook_output(&input, event)?;
            let reason = if event == "PermissionRequest" {
                output["hookSpecificOutput"]["decision"]["message"].as_str()
            } else {
                output["hookSpecificOutput"]["permissionDecisionReason"].as_str()
            }
            .ok_or("worktree denial reason")?;
            assert!(reason.contains("PARENT_WORKTREE"), "{reason}");
            assert!(reason.contains("handoff_thread"), "{reason}");
            assert!(reason.contains("primary checkout"), "{reason}");
        }
    }
    Ok(())
}

#[test]
fn verified_primary_checkout_admits_the_assigned_worker_pair() -> TestResult {
    let (_temp, primary, _worktree) = git_worktree_fixture()?;
    for tool in TOOLS {
        for event in ["PreToolUse", "PermissionRequest"] {
            let mut input = pre_tool_input_at(
                tool,
                json!({"model":"gpt-6-luna","thinking":"max"}),
                &primary,
            );
            input["hook_event_name"] = json!(event);
            assert!(!hook_denied_for(&input, event)?, "{event}: {input}");
        }
    }
    Ok(())
}

#[test]
fn primary_subdirectories_allow_but_nested_worktrees_are_rejected() -> TestResult {
    let (_temp, primary, _outer_worktree) = git_worktree_fixture()?;
    let ordinary_subdirectory = primary.join("docs");
    std::fs::create_dir_all(&ordinary_subdirectory)?;

    let nested_worktree = primary.join(".codexy-worktrees/lane");
    add_worktree(&primary, &nested_worktree)?;
    let nested_subdirectory = nested_worktree.join("src");
    std::fs::create_dir_all(&nested_subdirectory)?;

    for tool in TOOLS {
        for event in ["PreToolUse", "PermissionRequest"] {
            let mut primary_input = pre_tool_input_at(
                tool,
                json!({"model":"gpt-6-luna","thinking":"max"}),
                &ordinary_subdirectory,
            );
            primary_input["hook_event_name"] = json!(event);
            assert!(
                !hook_denied_for(&primary_input, event)?,
                "ordinary primary subdirectory was denied: {primary_input}"
            );

            for cwd in [&nested_worktree, &nested_subdirectory] {
                let mut input = pre_tool_input_at(
                    tool,
                    json!({"model":"gpt-6-luna","thinking":"max"}),
                    cwd,
                );
                input["hook_event_name"] = json!(event);
                let output = hook_output(&input, event)?;
                let reason = if event == "PermissionRequest" {
                    output["hookSpecificOutput"]["decision"]["message"].as_str()
                } else {
                    output["hookSpecificOutput"]["permissionDecisionReason"].as_str()
                }
                .ok_or("nested worktree denial reason")?;
                assert!(reason.contains("PARENT_WORKTREE"), "{reason}");
            }
        }
    }
    Ok(())
}

#[test]
fn unverifiable_parent_checkout_is_rejected_before_worker_creation() -> TestResult {
    for tool in TOOLS {
        for event in ["PreToolUse", "PermissionRequest"] {
            let input = json!({
                "hook_event_name":event,
                "tool_name":tool,
                "tool_input":{"model":"gpt-6-luna","thinking":"max"}
            });
            let output = hook_output(&input, event)?;
            let reason = if event == "PermissionRequest" {
                output["hookSpecificOutput"]["decision"]["message"].as_str()
            } else {
                output["hookSpecificOutput"]["permissionDecisionReason"].as_str()
            }
            .ok_or("unverified parent denial reason")?;
            assert!(reason.contains("PARENT_CHECKOUT_UNVERIFIED"), "{reason}");
            assert!(reason.contains("primary Git checkout"), "{reason}");
        }
    }
    Ok(())
}

#[test]
fn tool_input_cannot_claim_primary_cwd_for_a_worktree_parent() -> TestResult {
    let (_temp, primary, worktree) = git_worktree_fixture()?;
    let input = json!({
        "hook_event_name":"PreToolUse",
        "tool_name":TOOLS[0],
        "cwd":worktree,
        "tool_input":{
            "model":"gpt-6-luna",
            "thinking":"max",
            "cwd":primary,
        }
    });
    assert_hook(&input, true)
}

fn add_worktree(primary: &Path, worktree: &Path) -> TestResult {
    std::fs::create_dir_all(worktree.parent().ok_or("nested worktree parent")?)?;
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(primary)
        .args(["worktree", "add", "--detach"])
        .arg(worktree)
        .arg("HEAD")
        .output()?;
    assert!(
        output.status.success(),
        "nested git worktree add failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}
