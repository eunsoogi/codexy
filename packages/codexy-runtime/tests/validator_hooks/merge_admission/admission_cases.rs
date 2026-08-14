use super::*;

#[test]
fn merge_admission_hook_admits_valid_message_and_authorization() -> TestResult {
    let root = github_plugin_root();
    let temp = tempfile::tempdir()?;
    let message = temp.path().join("message.txt");
    let authorization = temp.path().join("authorization.json");
    let state_file = temp.path().join("state.json");
    std::fs::write(
        &message,
        "fix(workflow): require intent (#128)\n\nFixes #503\n",
    )?;
    std::fs::write(&authorization, contract())?;
    std::fs::write(&state_file, state())?;
    let output = admission(&root, &message, &authorization, &state_file)?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn connector_merge_without_authoritative_state_is_denied() -> TestResult {
    assert_tool_case(
        &plugin_root(),
        "mcp__codex_apps__github_merge_pull_request",
        json!({
            "repository_full_name":"eunsoogi/codexy", "pr_number":128, "merge_method":"squash",
            "expected_head_sha":"32b03a210b3defb2d29dd352283ea2488e60d893",
            "commit_title":"fix(workflow): require intent (#128)", "commit_message":"Fixes #503"
        }),
        true,
    )
}

#[cfg(unix)]
#[test]
fn canonical_wrapper_rejects_caller_authorization_state_paths() -> TestResult {
    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let owned = super::super::admission_runtime::repository(
        workspace.path(),
        "owned",
        "git@github.com:eunsoogi/codexy.git",
    )?;
    let message = owned.join("message.txt");
    let authorization = owned.join("authorization.json");
    let state_file = owned.join("state.json");
    let body = owned.join("body.txt");
    std::fs::write(
        &message,
        "fix(workflow): require intent (#128)\n\nFixes #503\n",
    )?;
    std::fs::write(&authorization, contract())?;
    std::fs::write(&state_file, state())?;
    std::fs::write(&body, "Fixes #503\n")?;
    let fake_bin = workspace.path().join("bin");
    std::fs::create_dir(&fake_bin)?;
    let fake_gh = fake_bin.join("gh");
    std::fs::write(
        &fake_gh,
        "#!/bin/sh\nprintf '%s\\n' \"$@\" > \"$CODEXY_GH_RECORD\"\n",
    )?;
    make_executable(&fake_gh)?;
    let record = workspace.path().join("gh-record.txt");
    let wrapper = github_plugin_root().join("hooks/codexy-authorized-squash-merge.sh");
    let command = format!(
        "{} --expected-pr 128 --expected-issue 503 --merge-message-file {} --merge-authorization-file {} --merge-authorization-pr-state-file {} --repo eunsoogi/codexy --match-head-commit 32b03a210b3defb2d29dd352283ea2488e60d893 --subject 'fix(workflow): require intent (#128)' --body-file {}",
        wrapper.display(),
        message.display(),
        authorization.display(),
        state_file.display(),
        body.display()
    );
    super::super::admission_runtime::assert_case(&root, &owned, &command, false, &[])?;
    super::super::admission_runtime::assert_case(
        &root,
        &owned,
        "gh pr merge 128 --repo eunsoogi/codexy --squash --match-head-commit 32b03a210b3defb2d29dd352283ea2488e60d893 --subject 'fix(workflow): require intent (#128)' --body-file body.txt",
        true,
        &[],
    )?;
    let path = format!("{}:{}", fake_bin.display(), std::env::var("PATH")?);
    let output = Command::new(&wrapper)
        .current_dir(&owned)
        .env("PATH", path)
        .env("CODEXY_GH_RECORD", &record)
        .args([
            "--expected-pr",
            "128",
            "--expected-issue",
            "503",
            "--merge-message-file",
        ])
        .arg(&message)
        .args(["--merge-authorization-file"])
        .arg(&authorization)
        .args(["--merge-authorization-pr-state-file"])
        .arg(&state_file)
        .args([
            "--repo",
            "eunsoogi/codexy",
            "--match-head-commit",
            "32b03a210b3defb2d29dd352283ea2488e60d893",
            "--subject",
            "fix(workflow): require intent (#128)",
            "--body-file",
        ])
        .arg(&body)
        .output()?;
    assert!(
        !output.status.success(),
        "caller-owned authorization state reached gh: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !record.exists(),
        "caller-owned authorization state reached gh"
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn canonical_wrapper_fetches_authorization_from_github_before_merging() -> TestResult {
    let root = github_plugin_root();
    let workspace = tempfile::tempdir()?;
    let owned = super::super::admission_runtime::repository(
        workspace.path(),
        "owned",
        "git@github.com:eunsoogi/codexy.git",
    )?;
    let message = owned.join("message.txt");
    let body = owned.join("body.txt");
    std::fs::write(
        &message,
        "fix(workflow): require intent (#128)\n\nFixes #503\n",
    )?;
    std::fs::write(&body, "Fixes #503\n")?;
    let fake_bin = workspace.path().join("bin");
    std::fs::create_dir(&fake_bin)?;
    let fake_gh = fake_bin.join("gh");
    std::fs::write(
        &fake_gh,
        "#!/bin/sh\nif [ \"$1\" = api ]; then cat \"$CODEXY_GH_STATE\"; else printf '%s\\n' \"$@\" > \"$CODEXY_GH_RECORD\"; fi\n",
    )?;
    make_executable(&fake_gh)?;
    let state_file = workspace.path().join("github-state.json");
    let record = workspace.path().join("gh-record.txt");
    std::fs::write(
        &state_file,
        state().replace(
            "AUTHORIZE REPOSITORY SQUASH CONTRACT",
            "AUTHORIZE SQUASH MERGE",
        ),
    )?;
    let output = Command::new(root.join("hooks/codexy-authorized-squash-merge.sh"))
        .current_dir(&owned)
        .env(
            "PATH",
            format!("{}:{}", fake_bin.display(), std::env::var("PATH")?),
        )
        .env("CODEXY_GH_STATE", &state_file)
        .env("CODEXY_GH_RECORD", &record)
        .args([
            "--expected-pr",
            "128",
            "--expected-issue",
            "503",
            "--merge-message-file",
        ])
        .arg(&message)
        .args([
            "--repo",
            "eunsoogi/codexy",
            "--match-head-commit",
            "32b03a210b3defb2d29dd352283ea2488e60d893",
            "--subject",
            "fix(workflow): require intent (#128)",
            "--body-file",
        ])
        .arg(&body)
        .output()?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        std::fs::read_to_string(record)?
            .lines()
            .take(2)
            .collect::<Vec<_>>(),
        ["pr", "merge"]
    );
    Ok(())
}
