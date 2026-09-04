use super::{TestResult, assert_input, plugin_root, repository};
use serde_json::{Value, json};

#[test]
fn nested_exec_github_mutations_use_the_repository_admission_route() -> TestResult {
    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let cwd = repository(workspace.path(), "owned", "git@github.com:eunsoogi/codexy.git")?;
    let cases = [
        (
            "valid nested issue",
            r#"await tools.mcp__codex_apps__github_create_issue({repository_full_name:"eunsoogi/codexy", title:"Create nested issue safely"});"#,
            false,
        ),
        (
            "invalid nested issue title",
            r#"await tools.mcp__codex_apps__github_create_issue({repository_full_name:"eunsoogi/codexy", title:"fix(hooks): bypass title policy"});"#,
            true,
        ),
        (
            "valid nested PR",
            r#"await tools.mcp__codex_apps__github_create_pull_request({repository_full_name:"eunsoogi/codexy", title:"fix(hooks): create nested PR safely", head_branch:"topic", base_branch:"main"});"#,
            false,
        ),
        (
            "invalid nested PR title",
            r#"await tools.mcp__codex_apps__github_create_pull_request({repository_full_name:"eunsoogi/codexy", title:"Create nested PR safely", head_branch:"topic", base_branch:"main"});"#,
            true,
        ),
        (
            "dynamic nested mutation",
            r#"const title = getTitle(); await tools.mcp__codex_apps__github_create_issue({repository_full_name:"eunsoogi/codexy", title});"#,
            true,
        ),
        (
            "ambiguous nested mutation arguments",
            r#"await tools.mcp__codex_apps__github_create_issue(getIssueInput());"#,
            true,
        ),
        (
            "read-only nested GitHub call",
            r#"await tools.mcp__codex_apps__github_get_repo({repository_full_name:"eunsoogi/codexy"});"#,
            false,
        ),
        (
            "unrelated functions.exec code",
            r#"const result = await doSomething(); text(result);"#,
            false,
        ),
        (
            "valid nested issue metadata",
            r#"await tools.mcp__codex_apps__github_create_issue({repository_full_name:"eunsoogi/codexy", title:"Create nested issue with metadata", body:"details", assignees:["eunsoogi"], labels:["type/fix"], milestone:35});"#,
            false,
        ),
        (
            "invalid nested issue metadata",
            r#"await tools.mcp__codex_apps__github_create_issue({repository_full_name:"eunsoogi/codexy", title:"Create nested issue with invalid metadata", milestone:0});"#,
            true,
        ),
        (
            "valid nested PR metadata",
            r#"await tools.mcp__codex_apps__github_create_pull_request({repository_full_name:"eunsoogi/codexy", title:"fix(hooks): create nested PR with metadata", head_branch:"topic", base_branch:"main", body:"details", draft:true, maintainer_can_modify:false, head_repo:"eunsoogi/codexy"});"#,
            false,
        ),
        (
            "invalid nested PR metadata",
            r#"await tools.mcp__codex_apps__github_create_pull_request({repository_full_name:"eunsoogi/codexy", title:"fix(hooks): reject nested PR metadata", head_branch:"", base_branch:"main"});"#,
            true,
        ),
        (
            "foreign nested mutation",
            r#"await tools.mcp__codex_apps__github_create_issue({repository_full_name:"openai/codex", title:"Create foreign nested issue"});"#,
            true,
        ),
        (
            "spread nested mutation",
            r#"await tools.mcp__codex_apps__github_create_issue({repository_full_name:"eunsoogi/codexy", ...issueInput});"#,
            true,
        ),
        (
            "computed nested mutation",
            r#"await tools["mcp__codex_apps__github_create_issue"]({repository_full_name:"eunsoogi/codexy", title:"Create computed nested issue"});"#,
            true,
        ),
        (
            "aliased nested mutation",
            r#"const create = tools.mcp__codex_apps__github_create_issue; await create({repository_full_name:"eunsoogi/codexy", title:"Create aliased nested issue"});"#,
            true,
        ),
        (
            "unknown nested mutation",
            r#"await tools.mcp__codex_apps__github_future_mutation({repository_full_name:"eunsoogi/codexy", title:"Create unknown nested mutation"});"#,
            true,
        ),
        (
            "optional computed nested mutation",
            r#"await tools?.[fullName]({repository_full_name:"eunsoogi/codexy", title:"Create optional nested issue"});"#,
            true,
        ),
        (
            "concatenated computed nested mutation",
            r#"await tools["mcp__codex_apps__" + "github_create_issue"]({repository_full_name:"eunsoogi/codexy", title:"Create concatenated nested issue"});"#,
            true,
        ),
        (
            "parenthesized Reflect.get mutation",
            r#"(Reflect.get)(tools, "mcp__codex_apps__github_create_issue")({repository_full_name:"eunsoogi/codexy", title:"Create reflected nested issue"});"#,
            true,
        ),
        (
            "literal eval nested mutation",
            r#"eval("await tools.mcp__codex_apps__github_create_issue({repository_full_name:\"eunsoogi/codexy\", title:\"Create evaluated nested issue\"});");"#,
            true,
        ),
        (
            "template expression nested mutation",
            r#"`${tools.mcp__codex_apps__github_create_issue({repository_full_name:"eunsoogi/codexy", title:"Create templated nested issue"})}`"#,
            true,
        ),
        (
            "GitHub names in comments and strings",
            r#"// tools.mcp__codex_apps__github_create_issue({title:"not a call"}); text("mcp__codex_apps__github_create_issue");"#,
            false,
        ),
        (
            "GitHub name in regex literal",
            r#"const pattern = /mcp__codex_apps__github_create_issue/;"#,
            false,
        ),
        (
            "one invalid nested call denies all",
            r#"await tools.mcp__codex_apps__github_get_repo({repository_full_name:"eunsoogi/codexy"}); await tools.mcp__codex_apps__github_create_issue({repository_full_name:"eunsoogi/codexy", title:"fix(hooks): invalid nested issue"});"#,
            true,
        ),
        (
            "unrelated nested tool call",
            r#"await tools.some_other_tool({value:getValue()});"#,
            false,
        ),
    ];
    for event in ["PermissionRequest", "PreToolUse"] {
        for (case_id, code, denied) in cases {
            assert_input(
                &root,
                json!({
                    "hook_event_name": event,
                    "tool_name": "functions.exec",
                    "tool_input": {"code": code},
                    "cwd": cwd,
                }),
                denied,
                &[],
            )
            .map_err(|error| format!("{case_id} {event}: {error}"))?;
        }
    }
    Ok(())
}

#[test]
fn nested_exec_uninspectable_denial_directs_to_direct_admitted_surface() -> TestResult {
    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let cwd = repository(workspace.path(), "owned", "git@github.com:eunsoogi/codexy.git")?;
    for event in ["PermissionRequest", "PreToolUse"] {
        let input = json!({
            "hook_event_name": event,
            "tool_name": "functions.exec",
            "tool_input": {"code": "await tools.mcp__codex_apps__github_create_issue(getIssueInput());"},
            "cwd": cwd,
        });
        let output = super::super::concern_launchers::run_launcher(
            &root,
            "codexy-repository-github-exec",
            event,
            &input,
            &[],
        )?;
        let denial: Value = serde_json::from_slice(&output)?;
        let reason = if event == "PermissionRequest" {
            denial["hookSpecificOutput"]["decision"]["message"]
                .as_str()
                .ok_or("permission reason")?
        } else {
            denial["hookSpecificOutput"]["permissionDecisionReason"]
                .as_str()
                .ok_or("pre-tool reason")?
        };
        assert!(
            reason.contains(
                "CODEXY_NESTED_GITHUB_UNINSPECTABLE_USE_DIRECT_ADMITTED_SURFACE"
            ),
            "{event}: {reason}"
        );
    }
    Ok(())
}
