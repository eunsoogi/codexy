use super::admission_runtime::{
    TestResult, assert_case, assert_event_case, executable, plugin_root, repository,
};
use crate::support::fixture_hook_path::modeled_path_token;
use std::path::{Path, PathBuf};

#[test]
fn issue_735_read_only_github_and_git_corpus_is_admitted_for_both_events() -> TestResult {
    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let owned = repository(workspace.path(), "owned", "git@github.com:eunsoogi/codexy.git")?;
    let commands = [
        "gh api repos/eunsoogi/codexy/labels --paginate --jq '.[].name'",
        "gh api repos/eunsoogi/codexy/assignees/eunsoogi --jq '.login'",
        "gh api repos/eunsoogi/codexy/branches/main/protection --jq '.required_pull_request_reviews'",
        "gh api repos/eunsoogi/codexy/milestones/23 --jq '.title'",
        "gh issue list --repo eunsoogi/codexy --limit 100",
        "gh label list --repo eunsoogi/codexy --limit 100",
        "gh pr list --repo eunsoogi/codexy --state all --limit 100",
        "gh release list --repo eunsoogi/codexy --limit 100",
        "git fetch origin main",
        "git rev-parse HEAD",
        "git worktree list",
        "git branch --list",
        "git ls-remote --heads origin",
        "git check-ref-format --branch topic",
        "for f in plugins/codexy/hooks/codexy_policy/child_thread_creation.py plugins/codexy/hooks/codexy_policy/thread_delivery.py plugins/codexy-github/hooks/codexy_policy/destructive_command.py plugins/codexy-github/hooks/codexy_policy/repository_github_command.py plugins/codexy-github/hooks/codexy_policy/repository_issue.py plugins/codexy-github/hooks/codexy_policy/repository_pull_request.py plugins/codexy-github/hooks/codexy_policy/repository_merge.py plugins/codexy-github/hooks/codexy_policy/titles.py plugins/codexy-github/hooks/codexy_policy/merge.py plugins/codexy-github/hooks/codexy_policy/pull_request.py; do echo \"### $f\"; git show eb34ef4f0292701b544bb73381d3c10a6b72d522:$f; done",
    ];
    for event in ["PermissionRequest", "PreToolUse"] {
        for command in commands {
            assert_event_case(&root, event, &owned, command, false, &[])?;
        }
    }
    Ok(())
}

#[test]
fn issue_735_closed_cli_and_rest_mutation_matrix_has_one_eligible_operation() -> TestResult {
    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let owned = repository(workspace.path(), "owned", "git@github.com:eunsoogi/codexy.git")?;
    let eligible = [
        "gh issue create --repo eunsoogi/codexy --title 'Valid issue' --body note --label bug --assignee eunsoogi --milestone 23",
        "gh issue edit 17 --repo eunsoogi/codexy --title 'Updated issue' --body note",
        "gh issue close 17 --repo eunsoogi/codexy --reason completed",
        "gh issue reopen 17 --repo eunsoogi/codexy",
        "gh issue comment 17 --repo eunsoogi/codexy --body note",
        "gh issue edit 17 --repo eunsoogi/codexy --add-label bug --remove-label old",
        "gh issue edit 17 --repo eunsoogi/codexy --add-assignee eunsoogi --remove-assignee old",
        "gh issue edit 17 --repo eunsoogi/codexy --milestone 23",
        "gh pr create --repo eunsoogi/codexy --title 'fix(hooks): admit safe mutations' --head topic --base main --body note --draft --no-maintainer-edit",
        "gh pr edit 17 --repo eunsoogi/codexy --title 'fix(hooks): update metadata' --body note --base main --no-maintainer-edit",
        "gh pr close 17 --repo eunsoogi/codexy",
        "gh pr reopen 17 --repo eunsoogi/codexy",
        "gh pr comment 17 --repo eunsoogi/codexy --body note",
        "gh pr review 17 --repo eunsoogi/codexy --approve --body LGTM",
        "gh pr edit 17 --repo eunsoogi/codexy --add-reviewer eunsoogi --remove-reviewer old",
        "gh pr ready 17 --repo eunsoogi/codexy --undo",
        "gh pr ready 17 --repo eunsoogi/codexy",
        "gh api --method POST repos/eunsoogi/codexy/issues -f title='Valid issue' -f body=note",
        "gh api --method PATCH repos/eunsoogi/codexy/issues/17 -f title='Updated issue' -f body=note",
        "gh api --method PATCH repos/eunsoogi/codexy/issues/17 -f state=closed -f state_reason=completed",
        "gh api --method POST repos/eunsoogi/codexy/issues/17/comments -f body=note",
        "gh api --method POST repos/eunsoogi/codexy/issues/17/labels -f labels=bug",
        "gh api --method POST repos/eunsoogi/codexy/issues/17/assignees -f assignees=eunsoogi",
        "gh api --method PATCH repos/eunsoogi/codexy/issues/17 -F milestone=23",
        "gh api --method POST repos/eunsoogi/codexy/pulls -f title='fix(hooks): create safe PR' -f head=topic -f base=main",
        "gh api --method PATCH repos/eunsoogi/codexy/pulls/17 -f title='fix(hooks): update metadata'",
        "gh api --method PATCH repos/eunsoogi/codexy/pulls/17 -f state=closed",
        "gh api --method POST repos/eunsoogi/codexy/pulls/17/requested_reviewers -f reviewers=eunsoogi",
        "gh api --method POST repos/eunsoogi/codexy/pulls/17/reviews -f event=APPROVE -f body=LGTM",
    ];
    for command in eligible {
        assert_case(&root, &owned, command, false, &[])?;
    }
    let denied = [
        "gh issue edit 17 --repo eunsoogi/codexy --title 'Updated issue' --add-label bug",
        "gh issue edit 17 --repo eunsoogi/codexy",
        "gh pr create --repo eunsoogi/codexy --title 'fix(hooks): missing base' --head topic",
        "gh pr edit 17 --repo eunsoogi/codexy --state closed",
        "gh pr merge 17 --repo eunsoogi/codexy --squash",
        "gh api --method POST repos/eunsoogi/codexy/import -f source=fixture",
        "gh api",
        "gh api --method POST repos/other-org/other-repo/issues -f title='Wrong repository'",
        "gh api --method POST repos/eunsoogi/codexy/issues/17 -f state=closed -f title='mixed operation'",
    ];
    for command in denied {
        assert_case(&root, &owned, command, true, &[])?;
    }
    Ok(())
}

#[test]
fn git_aliases_keep_the_normalized_repository_context() -> TestResult {
    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let owned = repository(workspace.path(), "owned", "git@github.com:eunsoogi/codexy.git")?;
    let foreign = repository(workspace.path(), "foreign", "https://github.com/openai/codex.git")?;
    for (cwd, target, denied) in [(&foreign, &owned, true), (&owned, &foreign, false)] {
        let git_dir = shell_path(&target.join(".git"))?;
        let target = shell_path(target)?;
        assert_case(
            &root, cwd,
            &format!("git -C {target} -c alias.wipe='!git reset --hard' wipe"),
            denied, &[],
        )?;
        assert_case(
            &root, cwd,
            &format!("git --git-dir={git_dir} -c alias.wipe='!git reset --hard' wipe"),
            denied, &[],
        )?;
    }
    Ok(())
}

#[test]
fn opaque_path_qualified_policy_executables_are_claimed() -> TestResult {
    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let owned = repository(workspace.path(), "owned", "git@github.com:eunsoogi/codexy.git")?;
    let git = executable("git")?;
    let gh = executable("gh")?;
    let printf = executable("printf")?;
    let renamed = workspace.path().join("renamed-tools");
    std::fs::create_dir(&renamed)?;
    copy_tool(&git, &renamed, "git-copy")?;
    copy_tool(&gh, &renamed, "gh-copy")?;
    let copied_printf = copy_tool(&printf, &renamed, "printf-copy")?;
    let wrappers = [
        ("command", ""),
        ("env", ""),
        ("exec", ""),
        ("sudo", ""),
        ("timeout", "1"),
        ("nohup", ""),
    ];
    let supported = "command ".repeat(7);
    let exhausted = "command ".repeat(8);
    let beyond = "command ".repeat(9);
    for event in ["PermissionRequest", "PreToolUse"] {
        assert_event_case(&root, event, &owned, &format!("if true; then '{}' reset --hard; fi", git.display()), true, &[])?;
        assert_event_case(&root, event, &owned, &format!("if true; then '{}' pr merge 551; fi", gh.display()), true, &[])?;
        assert_event_case(&root, event, &owned, &format!("if true; then PATH='{}' git-copy reset --hard; fi", renamed.display()), true, &[])?;
        assert_event_case(&root, event, &owned, &format!("if true; then PATH='{}' gh-copy pr merge 551; fi", renamed.display()), true, &[])?;
        assert_event_case(&root, event, &owned, &format!("if true; then ! PATH='{}' git-copy reset --hard; fi", renamed.display()), true, &[])?;
        assert_event_case(&root, event, &owned, &format!("if true; then ! PATH='{}' gh-copy pr merge 551; fi", renamed.display()), true, &[])?;
        assert_event_case(&root, event, &owned, "if true; then PATH=\"$UNKNOWN_RUNTIME_VALUE\" git-copy reset --hard; fi", true, &[])?;
        assert_event_case(&root, event, &owned, "if true; then PATH=\"$UNKNOWN_RUNTIME_VALUE\" gh-copy pr merge 551; fi", true, &[])?;
        assert_event_case(&root, event, &owned, "if true; then ! PATH=\"$UNKNOWN_RUNTIME_VALUE\" printf '%s\\n' safe; fi", true, &[])?;
        assert_event_case(&root, event, &owned, &format!("if true; then sudo -i '{}' reset --hard; fi", git.display()), true, &[])?;
        assert_event_case(&root, event, &owned, &format!("if true; then sudo -i '{}' pr merge 551; fi", gh.display()), true, &[])?;
        assert_event_case(&root, event, &owned, &format!("if true; then builtin command '{}' reset --hard; fi", git.display()), true, &[])?;
        assert_event_case(&root, event, &owned, &format!("if true; then builtin command '{}' pr merge 551; fi", gh.display()), true, &[])?;
        assert_event_case(&root, event, &owned, "if true; then command -v printf; fi", false, &[])?;
        assert_event_case(&root, event, &owned, "if true; then builtin command -v printf; fi", false, &[])?;
        assert_event_case(&root, event, &owned, &format!("if true; then {supported}'{}' '%s\\n' safe; fi", copied_printf.display()), false, &[])?;
        for nested in [&exhausted, &beyond] {
            assert_event_case(&root, event, &owned, &format!("if true; then {nested}'{}' reset --hard; fi", git.display()), true, &[])?;
            assert_event_case(&root, event, &owned, &format!("if true; then {nested}'{}' pr merge 551; fi", gh.display()), true, &[])?;
        }
        for (wrapper, option) in wrappers {
            assert_event_case(&root, event, &owned, &format!("if true; then {wrapper} {option} '{}' reset --hard; fi", git.display()), true, &[])?;
            assert_event_case(&root, event, &owned, &format!("if true; then {wrapper} {option} '{}' pr merge 551; fi", gh.display()), true, &[])?;
            assert_event_case(&root, event, &owned, &format!("if true; then PATH=\"$UNKNOWN_RUNTIME_VALUE\" {wrapper} {option} '{}' '%s\\n' safe; fi", copied_printf.display()), true, &[])?;
        }
        assert_event_case(&root, event, &owned, &format!("if true; then printf '%s\\n' '{}'; fi", git.display()), false, &[])?;
        assert_event_case(&root, event, &owned, &format!("if true; then printf '%s\\n' '{}'; fi", gh.display()), false, &[])?;
        assert_event_case(&root, event, &owned, &format!("if true; then '{}' reset --hard; fi", printf.display()), false, &[])?;
    }
    Ok(())
}

#[test]
fn opaque_protected_arguments_and_unreachable_controls_preserve_policy() -> TestResult {
    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let owned = repository(workspace.path(), "owned", "git@github.com:eunsoogi/codexy.git")?;
    let foreign = repository(workspace.path(), "foreign", "https://github.com/openai/codex.git")?;
    for event in ["PermissionRequest", "PreToolUse"] {
        assert_event_case(&root, event, &owned, "gh issue \"$ACTION\"", true, &[])?;
        assert_event_case(&root, event, &owned, "printf \"$ACTION\"", false, &[])?;
        assert_event_case(
            &root,
            event,
            &owned,
            &format!(
                "if false; then cd {}; fi; gh issue create --title invalid",
                foreign.display()
            ),
            true,
            &[],
        )?;
        assert_event_case(
            &root,
            event,
            &owned,
            &format!("if false; then cd {}; fi; gh repo view", foreign.display()),
            false,
            &[],
        )?;
    }
    Ok(())
}

fn shell_path(path: &Path) -> TestResult<String> {
    modeled_path_token(path.to_str().ok_or("path")?, &|value| Ok(value.to_owned()))?
        .ok_or_else(|| "absolute shell path".into())
}

fn copy_tool(source: &Path, directory: &Path, name: &str) -> TestResult<PathBuf> {
    let mut destination = directory.join(name);
    if let Some(extension) = source.extension() {
        destination.set_extension(extension);
    }
    std::fs::copy(source, &destination)?;
    Ok(destination)
}
