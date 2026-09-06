use super::body_cases::{MINIMAL_INVALID_BODY, VALID_BODY};
use super::super::admission_runtime::{
    TestResult, assert_event_case, assert_input, plugin_root, repository,
};
use serde_json::{Value, json};
use std::path::Path;

#[test]
fn pr_body_contract_is_shared_across_registered_mutation_routes() -> TestResult {
    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let cwd = repository(workspace.path(), "owned", "git@github.com:eunsoogi/codexy.git")?;
    for body in [VALID_BODY, MINIMAL_INVALID_BODY] {
        for (tool, create) in [
            ("mcp__codex_apps__github_create_pull_request", true),
            ("github.create_pull_request", true),
            ("mcp__codex_apps__github_update_pull_request", false),
            ("github.update_pull_request", false),
        ] {
            assert_connector(
                &root,
                &cwd,
                "connector body route",
                tool,
                create,
                body,
                body == MINIMAL_INVALID_BODY,
            )?;
        }
        assert_nested(
            &root,
            &cwd,
            "nested body route",
            body,
            body == MINIMAL_INVALID_BODY,
        )?;
        for command in shell_commands(body) {
            assert_shell(
                &root,
                &cwd,
                "shell body route",
                &command,
                body == MINIMAL_INVALID_BODY,
            )?;
        }
        for command in graphql_commands(body)? {
            assert_shell(
                &root,
                &cwd,
                "GraphQL body route",
                &command,
                body == MINIMAL_INVALID_BODY,
            )?;
        }
    }
    let command = graphql_create_without_body();
    assert_shell(
        &root,
        &cwd,
        "GraphQL create without body",
        &command,
        true,
    )?;
    assert_metadata_only_updates(&root, &cwd)?;
    Ok(())
}

fn assert_metadata_only_updates(root: &Path, cwd: &Path) -> TestResult {
    for (tool, input) in [
        (
            "mcp__codex_apps__github_update_pull_request",
            json!({"repository_full_name":"eunsoogi/codexy","pr_number":17,"title":"fix(hooks): title-only update"}),
        ),
        (
            "mcp__codex_apps__github_update_pull_request",
            json!({"repository_full_name":"eunsoogi/codexy","pr_number":17,"base":"main"}),
        ),
        (
            "github.update_pull_request",
            json!({"repository_full_name":"eunsoogi/codexy","pr_number":17,"title":"fix(hooks): normalized title-only update"}),
        ),
        (
            "github.update_pull_request",
            json!({"repository_full_name":"eunsoogi/codexy","pr_number":17,"base":"main"}),
        ),
    ] {
        assert_connector_input(root, cwd, tool, input, false)?;
    }
    for code in [
        r#"await tools.mcp__codex_apps__github_update_pull_request({repository_full_name:"eunsoogi/codexy",pr_number:17,title:"fix(hooks): nested title-only update"});"#,
        r#"await tools.mcp__codex_apps__github_update_pull_request({repository_full_name:"eunsoogi/codexy",pr_number:17,base_branch:"main"});"#,
    ] {
        for event in ["PermissionRequest", "PreToolUse"] {
            assert_input(
                root,
                json!({"hook_event_name":event,"tool_name":"functions.exec","tool_input":{"code":code},"cwd":cwd}),
                false,
                &[],
            )?;
        }
    }
    for command in [
        "gh pr edit 17 --repo eunsoogi/codexy --title 'fix(hooks): cli title-only update'",
        "gh pr edit 17 --repo eunsoogi/codexy --base main",
        "gh api repos/eunsoogi/codexy/pulls/17 -X PATCH -f title='fix(hooks): REST title-only update'",
        "gh api repos/eunsoogi/codexy/pulls/17 -X PATCH -f base=main",
        "gh api graphql -f owner=eunsoogi -f name=codexy -f repository_id=R_kgDOS6i-_w -f pull_request_id=PR_kwDOS6i-_88AAAABBJnhRQ -f query='mutation { updatePullRequest(input:{pullRequestId:\"PR_kwDOS6i-_88AAAABBJnhRQ\",title:\"fix(hooks): GraphQL title-only update\"}) { pullRequest { number } } }'",
        "gh api graphql -f owner=eunsoogi -f name=codexy -f repository_id=R_kgDOS6i-_w -f pull_request_id=PR_kwDOS6i-_88AAAABBJnhRQ -f query='mutation { updatePullRequest(input:{pullRequestId:\"PR_kwDOS6i-_88AAAABBJnhRQ\",baseRefName:\"main\"}) { pullRequest { number } } }'",
    ] {
        assert_shell(root, cwd, "metadata-only update", command, false)?;
    }
    Ok(())
}

fn assert_connector(
    root: &Path,
    cwd: &Path,
    case_id: &str,
    tool: &str,
    create: bool,
    body: &str,
    denied: bool,
) -> TestResult {
    let input = if create {
        json!({"repository_full_name":"eunsoogi/codexy","title":"fix(hooks): body route","head_branch":"topic","base_branch":"main","body":body})
    } else {
        json!({"repository_full_name":"eunsoogi/codexy","pr_number":17,"body":body})
    };
    assert_connector_input(root, cwd, tool, input, denied).map_err(|error| {
        format!("{case_id} connector {tool}: {error}").into()
    })
}

fn assert_connector_input(
    root: &Path,
    cwd: &Path,
    tool: &str,
    input: Value,
    denied: bool,
) -> TestResult {
    for event in ["PermissionRequest", "PreToolUse"] {
        assert_input(
            root,
            json!({"hook_event_name":event,"tool_name":tool,"tool_input":input.clone(),"cwd":cwd}),
            denied,
            &[],
        )?;
    }
    Ok(())
}

fn assert_nested(
    root: &Path,
    cwd: &Path,
    case_id: &str,
    body: &str,
    denied: bool,
) -> TestResult {
    let body = serde_json::to_string(body)?;
    let code = format!(
        r#"await tools.mcp__codex_apps__github_create_pull_request({{repository_full_name:"eunsoogi/codexy",title:"fix(hooks): nested body route",head_branch:"topic",base_branch:"main",body:{body}}});"#
    );
    for event in ["PermissionRequest", "PreToolUse"] {
        assert_input(
            root,
            json!({"hook_event_name":event,"tool_name":"functions.exec","tool_input":{"code":code},"cwd":cwd}),
            denied,
            &[],
        )
        .map_err(|error| format!("{case_id} nested {event}: {error}"))?;
    }
    Ok(())
}

fn shell_commands(body: &str) -> [String; 4] {
    let body = shell_quote(body);
    [
        format!("gh pr create --repo eunsoogi/codexy --title 'fix(hooks): cli body route' --head topic --base main --body {body}"),
        format!("gh pr edit 17 --repo eunsoogi/codexy --body {body}"),
        format!("gh api repos/eunsoogi/codexy/pulls -X POST -f title='fix(hooks): REST body route' -f head=topic -f base=main -f body={body}"),
        format!("gh api repos/eunsoogi/codexy/pulls/17 -X PATCH -f body={body}"),
    ]
}

fn graphql_commands(body: &str) -> TestResult<[String; 2]> {
    let body = serde_json::to_string(body)?;
    let create = format!(
        r#"mutation {{ createPullRequest(input:{{repositoryId:"R_kgDOS6i-_w",title:"fix(hooks): GraphQL body route",headRefName:"topic",baseRefName:"main",body:{body}}}) {{ pullRequest {{ number }} }} }}"#
    );
    let update = format!(
        r#"mutation {{ updatePullRequest(input:{{pullRequestId:"PR_kwDOS6i-_88AAAABBJnhRQ",body:{body}}}) {{ pullRequest {{ number }} }} }}"#
    );
    let prefix = "gh api graphql -f owner=eunsoogi -f name=codexy -f repository_id=R_kgDOS6i-_w -f pull_request_id=PR_kwDOS6i-_88AAAABBJnhRQ -f query=";
    Ok([
        format!("{prefix}{}", shell_quote(&create)),
        format!("{prefix}{}", shell_quote(&update)),
    ])
}

fn graphql_create_without_body() -> String {
    let query = r#"mutation { createPullRequest(input:{repositoryId:"R_kgDOS6i-_w",title:"fix(hooks): GraphQL body route",headRefName:"topic",baseRefName:"main"}) { pullRequest { number } } }"#;
    let prefix = "gh api graphql -f owner=eunsoogi -f name=codexy -f repository_id=R_kgDOS6i-_w -f query=";
    format!("{prefix}{}", shell_quote(query))
}

fn assert_shell(
    root: &Path,
    cwd: &Path,
    case_id: &str,
    command: &str,
    denied: bool,
) -> TestResult {
    for event in ["PermissionRequest", "PreToolUse"] {
        assert_event_case(root, event, cwd, command, denied, &[])
            .map_err(|error| format!("{case_id} {event} {command}: {error}"))?;
    }
    Ok(())
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}
