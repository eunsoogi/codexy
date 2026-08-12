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
        "Child handoff: branch clean, synced, and pushed at 068dbb247b7755035223c91ee39f26830f3c1609. Packaged Codexy Sentinel Turing: PASS on current head 068dbb247b7755035223c91ee39f26830f3c1609.\n",
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
            "localHeadOid": "068dbb247b7755035223c91ee39f26830f3c1609",
            "remoteHeadOid": "068dbb247b7755035223c91ee39f26830f3c1609",
            "reviewDecision": "NOT_REQUIRED",
            "reviewControl": {"schema":"codexy.review-control-state.v1","profile":"light","decision":"NOT_REQUIRED"},
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
