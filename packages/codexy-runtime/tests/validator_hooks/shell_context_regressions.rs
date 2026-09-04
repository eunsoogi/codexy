use super::admission_runtime::{
    TestResult, assert_case, assert_event_case, assert_event_cases, executable, plugin_root,
    repository,
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
        "gh api repos/eunsoogi/codexy/labels --paginate --jq '.[].name | test(\"git push|gh pr merge\")' > /dev/null",
        "gh api repos/eunsoogi/codexy/labels --paginate --jq '.[].name' 2>/dev/null",
        "gh api repos/eunsoogi/codexy/labels --paginate --jq '.[].name' | jq -r 'select(test(\"delete|merge\") | not)'",
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
        "gh api --method POST repos/eunsoogi/codexy/issues -f title='Valid issue' > /dev/null",
        "gh api --method POST repos/eunsoogi/codexy/issues -f title='Valid issue' -F 'labels=[\"bug\"]' -F 'assignees=[\"eunsoogi\"]'",
        "gh api --method POST repos/eunsoogi/codexy/issues -f title='Valid issue' -F 'labels[]=bug' -F 'labels[]=workflow'",
        "gh api --method POST 'repos/{owner}/{repo}/issues' -f title='Valid issue'",
        "gh api --method PATCH repos/eunsoogi/codexy/issues/17 -f title='Updated issue' -f body=note",
        "gh api --method PATCH repos/eunsoogi/codexy/issues/17 -f state=closed -f state_reason=completed",
        "gh api --method POST repos/eunsoogi/codexy/issues/17/comments -f body=note",
        "gh api --method DELETE repos/eunsoogi/codexy/issues/17/labels/old",
        "gh api --method POST repos/eunsoogi/codexy/issues/17/labels -F 'labels=[\"bug\"]'",
        "gh api --method DELETE repos/eunsoogi/codexy/issues/17/assignees -F 'assignees=[\"old\"]'",
        "gh api --method POST repos/eunsoogi/codexy/issues/17/assignees -F 'assignees=[\"eunsoogi\"]'",
        "gh api --method PATCH repos/eunsoogi/codexy/issues/17 -F 'labels=[]'",
        "gh api --method PATCH repos/eunsoogi/codexy/issues/17 -F 'assignees=[]'",
        "gh api --method PATCH repos/eunsoogi/codexy/issues/17 -F milestone=23",
        "gh api --method POST repos/eunsoogi/codexy/pulls -f title='fix(hooks): create safe PR' -f head=topic -f base=main",
        "gh api --method PATCH repos/eunsoogi/codexy/pulls/17 -f title='fix(hooks): update metadata'",
        "gh api --method PATCH repos/eunsoogi/codexy/pulls/17 -f state=closed",
        "gh api --method POST repos/eunsoogi/codexy/pulls/17/requested_reviewers -F 'reviewers=[\"eunsoogi\"]'",
        "gh api --method POST repos/eunsoogi/codexy/pulls/17/requested_reviewers -F 'reviewers=[\"eunsoogi\"]' -F 'team_reviewers=[\"codexy\"]'",
        "gh api --method POST repos/eunsoogi/codexy/pulls/17/requested_reviewers -F 'reviewers[]=eunsoogi' -F 'reviewers[]=reviewer'",
        "gh api --method DELETE repos/eunsoogi/codexy/pulls/17/requested_reviewers -F 'reviewers=[\"old\"]'",
        "gh api --method POST repos/eunsoogi/codexy/pulls/17/reviews -f event=APPROVE -f body=LGTM",
    ];
    let denied = [
        "gh api --method DELETE repos/eunsoogi/codexy",
        "gh api --method DELETE repos/eunsoogi/codexy/git/refs/heads/topic",
        "gh api --method PUT repos/eunsoogi/codexy/branches/main/protection -f required_status_checks='{}'",
        "gh api --method PUT repos/eunsoogi/codexy/actions/secrets/BUILD_KEY -f encrypted_value=fixture",
        "gh api --method PUT repos/eunsoogi/codexy/collaborators/mallory -f permission=push",
        "gh api --method POST repos/eunsoogi/codexy/hooks -f name=fixture",
        "gh workflow run deploy.yml --repo eunsoogi/codexy",
        "gh release create v9 --repo eunsoogi/codexy",
        "gh api --method PATCH repos/eunsoogi/codexy/security-advisories/GHSA-fixture -f state=closed",
        "gh pr merge 42 --repo eunsoogi/codexy --squash",
        "gh api --method POST repos/eunsoogi/codexy/import -f source=fixture",
        "gh api graphql -f owner=eunsoogi -f name=codexy -f query='mutation { deleteProjectV2(input:{projectV2Id:\"fixture\"}) { clientMutationId } }'",
        "gh api --method POST repos/other-org/other-repo/issues -f title='Wrong repository'",
        "gh api -H 'Authorization: Bearer fixture-secret' repos/eunsoogi/codexy/labels",
        "rm -rf /",
        "gh pr edit 42 --repo eunsoogi/codexy --expected-head aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "gh api --method POST repos/eunsoogi/codexy/issues",
        "gh api --method POST repos/eunsoogi/codexy/pulls/17/reviews -f event=COMMENT",
        "gh api --method POST repos/eunsoogi/codexy/issues > /dev/null",
        "cat <(gh issue create --repo eunsoogi/codexy --title invalid)",
        "cat >(git push --force origin main)",
    ];
    let cases = eligible.iter()
        .map(|command| ((*command).to_owned(), false))
        .chain(denied.iter().map(|command| ((*command).to_owned(), true)))
        .collect::<Vec<_>>();
    for event in ["PermissionRequest", "PreToolUse"] {
        assert_event_cases(&root, event, &owned, cases.clone(), &[])?;
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
    let mut cases = vec![
        (format!("if true; then '{}' reset --hard; fi", git.display()), true),
        (format!("if true; then '{}' pr merge 551; fi", gh.display()), true),
        (format!("if true; then PATH='{}' git-copy reset --hard; fi", renamed.display()), true),
        (format!("if true; then PATH='{}' gh-copy pr merge 551; fi", renamed.display()), true),
        (format!("if true; then ! PATH='{}' git-copy reset --hard; fi", renamed.display()), true),
        (format!("if true; then ! PATH='{}' gh-copy pr merge 551; fi", renamed.display()), true),
        ("if true; then PATH=\"$UNKNOWN_RUNTIME_VALUE\" git-copy reset --hard; fi".to_owned(), true),
        ("if true; then PATH=\"$UNKNOWN_RUNTIME_VALUE\" gh-copy pr merge 551; fi".to_owned(), true),
        ("if true; then ! PATH=\"$UNKNOWN_RUNTIME_VALUE\" printf '%s\\n' safe; fi".to_owned(), true),
        (format!("if true; then sudo -i '{}' reset --hard; fi", git.display()), true),
        (format!("if true; then sudo -i '{}' pr merge 551; fi", gh.display()), true),
        (format!("if true; then builtin command '{}' reset --hard; fi", git.display()), true),
        (format!("if true; then builtin command '{}' pr merge 551; fi", gh.display()), true),
        ("if true; then command -v printf; fi".to_owned(), false),
        ("if true; then builtin command -v printf; fi".to_owned(), false),
        (format!("if true; then {supported}'{}' '%s\\n' safe; fi", copied_printf.display()), false),
    ];
    for nested in [&exhausted, &beyond] {
        cases.push((format!("if true; then {nested}'{}' reset --hard; fi", git.display()), true));
        cases.push((format!("if true; then {nested}'{}' pr merge 551; fi", gh.display()), true));
    }
    for (wrapper, option) in wrappers {
        cases.push((format!("if true; then {wrapper} {option} '{}' reset --hard; fi", git.display()), true));
        cases.push((format!("if true; then {wrapper} {option} '{}' pr merge 551; fi", gh.display()), true));
        cases.push((format!("if true; then PATH=\"$UNKNOWN_RUNTIME_VALUE\" {wrapper} {option} '{}' '%s\\n' safe; fi", copied_printf.display()), true));
    }
    cases.extend([
        (format!("if true; then printf '%s\\n' '{}'; fi", git.display()), false),
        (format!("if true; then printf '%s\\n' '{}'; fi", gh.display()), false),
        (format!("if true; then '{}' reset --hard; fi", printf.display()), false),
    ]);
    for event in ["PermissionRequest", "PreToolUse"] {
        assert_event_cases(&root, event, &owned, cases.clone(), &[])?;
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
