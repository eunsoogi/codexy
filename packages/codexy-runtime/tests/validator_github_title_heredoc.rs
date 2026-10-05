// Keep literal PR-body input separate from shell script input in the packaged hook.
use super::validator_github_title_hooks::{assert_title, TestResult};
use serde_json::json;

#[test]
fn quoted_body_heredocs_do_not_weaken_title_checks() -> TestResult {
    let body = "reviewer's body update\ngh pr create --title 'plain title'\n";
    let cases = [
        (format!("gh pr edit 1249 --body-file - <<'BODY'\n{body}BODY\n"), false),
        (format!("gh pr edit 1249 -F - <<'BODY'\n{body}BODY\n"), false),
        (
            format!("gh pr edit 1249 --body-file=- <<'BODY'\n{body}BODY\n"),
            false,
        ),
        (
            format!(
                "gh pr edit 1249 --title 'plain title' --body-file - <<'BODY'\n{body}BODY\n"
            ),
            true,
        ),
        (
            format!(
                "gh pr edit 1249 --title='plain title' --body-file - <<'BODY'\n{body}BODY\n"
            ),
            true,
        ),
        (
            format!("gh pr edit 1249 -t 'plain title' --body-file - <<'BODY'\n{body}BODY\n"),
            true,
        ),
        (
            format!(
                "gh pr edit 1249 --title 'fix(hooks): valid title' --body-file - <<'BODY'\n{body}BODY\n"
            ),
            false,
        ),
        (
            "sh -s <<'BODY'\ngh pr edit 1249 --title 'plain title'\nBODY\n".to_string(),
            true,
        ),
        (
            "sh -c 'sh -s' <<'BODY'\ngh pr edit 1249 --title 'plain title'\nBODY\n"
                .to_string(),
            true,
        ),
        (
            "gh pr edit 1249 --body-file - <<BODY\n$(gh pr create --title 'plain title')\nBODY\n"
                .to_string(),
            true,
        ),
        (
            "cat <<'BODY' | sh\ngh pr edit 1249 --title 'plain title'\nBODY\n".to_string(),
            true,
        ),
    ];
    for event in ["PermissionRequest", "PreToolUse"] {
        for (command, denied) in &cases {
            assert_title(
                event,
                "shell",
                json!({
                    "hook_event_name": event,
                    "tool_name": "Bash",
                    "tool_input": {"command": command},
                }),
                *denied,
            )?;
        }
    }
    Ok(())
}
