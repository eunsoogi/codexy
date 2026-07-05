#[test]
fn git_workflow_does_not_accept_thumbs_up_only_codex_completion()
-> Result<(), Box<dyn std::error::Error>> {
    let skill = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("plugins/codexy/skills/git-workflow/SKILL.md"),
    )?;

    assert!(
        !skill.contains("thumbs-up reaction"),
        "aggregate thumbs-up reactions do not prove the actor was Codex"
    );
    assert!(
        !skill.contains("such as `+1`"),
        "Codex completion signals should require connector-authored output, not bare reactions"
    );
    Ok(())
}

#[test]
fn git_workflow_fetches_inline_review_comment_commit_oids() -> Result<(), Box<dyn std::error::Error>>
{
    let reference = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("plugins/codexy/skills/git-workflow/references/pr-review-and-handoff.md"),
    )?;

    assert!(
        reference.contains("commit { oid }"),
        "reviewThreads comment evidence must include inline review comment commit OIDs"
    );
    assert!(
        reference.contains("git status --short --branch > \"$state_dir/worktreeStatus.txt\"")
            && reference.contains("worktreeStatus: $worktreeStatus"),
        "completion-handoff capture must include current local git status evidence"
    );
    Ok(())
}

#[test]
fn pr_review_handoff_capture_includes_branch_status_evidence()
-> Result<(), Box<dyn std::error::Error>> {
    let reference = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("plugins/codexy/skills/git-workflow/references/pr-review-and-handoff.md"),
    )?;

    assert!(
        reference.contains("state_dir=$(mktemp -d)"),
        "documented PR-state capture must keep scratch files out of the worktree"
    );
    assert!(
        reference.contains("git status --short --branch > \"$state_dir/worktreeStatus.txt\""),
        "documented PR-state capture must collect local branch status evidence"
    );
    assert!(
        reference.contains("--rawfile worktreeStatus \"$state_dir/worktreeStatus.txt\""),
        "documented PR-state assembly must read captured branch status"
    );
    assert!(
        reference.contains("worktreeStatus: $worktreeStatus"),
        "documented PR-state output must expose branch status to validators"
    );
    Ok(())
}

#[test]
fn pr_review_handoff_status_capture_does_not_dirty_clean_worktree()
-> Result<(), Box<dyn std::error::Error>> {
    let repo = tempfile::tempdir()?;
    let remote = tempfile::tempdir()?;
    let state = tempfile::tempdir()?;
    let status_path = state.path().join("worktreeStatus.txt");

    run_git(repo.path(), ["init", "-b", "codexy/example"])?;
    run_git(remote.path(), ["init", "--bare"])?;
    run_git(repo.path(), ["commit", "--allow-empty", "-m", "init"])?;
    run_git(
        repo.path(),
        [
            "remote",
            "add",
            "origin",
            remote.path().to_str().ok_or("remote path")?,
        ],
    )?;
    run_git(repo.path(), ["push", "-u", "origin", "codexy/example"])?;
    let status = std::process::Command::new("git")
        .args(["status", "--short", "--branch"])
        .current_dir(repo.path())
        .output()?;
    assert!(status.status.success(), "git status should succeed");
    std::fs::write(&status_path, &status.stdout)?;

    let status_text = std::fs::read_to_string(&status_path)?;
    assert!(status_text.starts_with("## "), "missing branch header");
    assert!(
        !status_text.contains("??"),
        "external status capture must not create untracked worktree evidence: {status_text}"
    );

    let handoff_path = state.path().join("handoff.md");
    let pr_state_path = state.path().join("pr-state.json");
    std::fs::write(
        &handoff_path,
        "Child handoff: branch clean, synced, and pushed at 068dbb247b7755035223c91ee39f26830f3c1609.\n",
    )?;
    std::fs::write(
        &pr_state_path,
        serde_json::json!({
            "number": 242,
            "state": "OPEN",
            "isDraft": false,
            "mergeStateStatus": "CLEAN",
            "headRefName": "codexy/example",
            "headRefOid": "068dbb247b7755035223c91ee39f26830f3c1609",
            "worktreeStatus": status_text,
            "reviewThreads": {"pageInfo":{"hasNextPage":false},"nodes":[]}
        })
        .to_string(),
    )?;

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--check-completion-handoff",
            "--handoff-file",
            handoff_path.to_str().ok_or("handoff path")?,
            "--pr-state-file",
            pr_state_path.to_str().ok_or("pr state path")?,
        ])
        .output()?;

    assert!(
        output.status.success(),
        "clean external branch-status evidence should validate\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

fn run_git<const N: usize>(
    cwd: &std::path::Path,
    args: [&str; N],
) -> Result<(), Box<dyn std::error::Error>> {
    let output = std::process::Command::new("git")
        .args([
            "-c",
            "user.name=Codexy Test",
            "-c",
            "user.email=codexy@example.invalid",
        ])
        .args(args)
        .current_dir(cwd)
        .output()?;
    assert!(
        output.status.success(),
        "git command should succeed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}
