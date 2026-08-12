use crate::support::{FixtureCommand as Command, make_executable};

#[allow(unused)]
use crate::support;

#[test]
fn installed_readiness_guard_validates_merge_bodies() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    support::copy_dir(
        codexy_runtime::paths::repository_root().join("plugins/codexy"),
        &plugin_root,
    )?;
    assert!(!plugin_root.join("scripts/validate-plugin-config").exists());
    let script = plugin_root.join("hooks/codexy-readiness-guard.sh");

    let bad = Command::new(&script)
        .args([
            "--check-merge-message",
            "--expected-pr",
            "204",
            "--merge-message",
            "fix(workflow): x (#204)\n\nCloses #999\n",
        ])
        .output()?;
    assert!(!bad.status.success());
    assert!(
        output_text(&bad).contains("merge commit message must not contain closing references"),
        "unexpected output: {}",
        output_text(&bad)
    );

    let good = Command::new(&script)
        .args([
            "--check-merge-message",
            "--expected-pr",
            "204",
            "--expected-issue",
            "206",
            "--merge-message",
            "fix(workflow): x (#204)\n\nFixes #206\n",
        ])
        .output()?;
    assert!(
        good.status.success(),
        "installed guard should accept valid merge messages\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&good.stdout),
        String::from_utf8_lossy(&good.stderr)
    );

    let message = write_pr_state(
        temp.path(),
        "message.txt",
        "fix(workflow): x (#204)\n\nFixes #206\n",
    )?;
    let body = write_pr_state(temp.path(), "body.txt", "Fixes #206\n")?;
    let state = write_pr_state(
        temp.path(),
        "state.json",
        r#"{"repository":"eunsoogi/codexy","number":204,"baseRefName":"main","headRefOid":"abc123","comments":[{"id":"IC_204","url":"https://github.com/eunsoogi/codexy/pull/204#issuecomment-204","body":"AUTHORIZE REPOSITORY SQUASH CONTRACT: PR #204 BASE main HEAD abc123","author":{"login":"maintainer"},"authorAssociation":"OWNER"}]}"#,
    )?;
    let bin = temp.path().join("bin");
    std::fs::create_dir(&bin)?;
    let gh = bin.join("gh");
    std::fs::write(&gh, "#!/bin/sh\nif [ \"$1\" = api ]; then cat \"$CODEXY_GH_STATE\"; else printf merge > \"$CODEXY_GH_RECORD\"; fi\n")?;
    make_executable(&gh)?;
    let record = temp.path().join("merge.txt");
    let wrapper = plugin_root.join("hooks/codexy-authorized-squash-merge.sh");
    let run = |valid: bool| -> Result<std::process::Output, Box<dyn std::error::Error>> {
        std::fs::remove_file(&record).ok();
        if !valid {
            std::fs::write(&state, "{\"repository\":\"eunsoogi/codexy\",\"number\":204,\"baseRefName\":\"main\",\"headRefOid\":\"abc123\",\"comments\":[]}")?;
        }
        Ok(Command::new(&wrapper)
            .env("PATH", format!("{}:{}", bin.display(), std::env::var("PATH")?))
            .env("CODEXY_GH_STATE", &state)
            .env("CODEXY_GH_RECORD", &record)
            .args(["--expected-pr", "204", "--expected-issue", "206", "--merge-message-file"])
            .arg(&message)
            .args(["--repo", "eunsoogi/codexy", "--match-head-commit", "abc123", "--subject", "fix(workflow): x (#204)", "--body-file"])
            .arg(&body)
            .output()?)
    };
    let authorized = run(true)?;
    assert!(authorized.status.success(), "installed wrapper failed: {}", output_text(&authorized));
    assert!(record.exists(), "installed wrapper did not reach the admitted merge");
    let denied = run(false)?;
    assert!(!denied.status.success(), "unauthorized installed merge was admitted");
    assert!(!record.exists(), "unauthorized installed merge reached gh");
    Ok(())
}

#[test]
fn installed_readiness_guard_validates_pr_labels() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    support::copy_dir(
        codexy_runtime::paths::repository_root().join("plugins/codexy"),
        &plugin_root,
    )?;
    assert!(!plugin_root.join("scripts/validate-plugin-config").exists());
    let script = plugin_root.join("hooks/codexy-readiness-guard.sh");
    let configured = temp.path().join("configured");
    std::fs::create_dir_all(configured.join(".git"))?;
    std::fs::write(configured.join(".git/config"), "[remote \"origin\"]\n\turl = git@github.com:eunsoogi/codexy.git\n")?;
    let policy = configured.join(".codex/repository-github-policy.json");
    std::fs::create_dir_all(policy.parent().ok_or("policy parent")?)?;
    std::fs::write(policy, "{\"schema\":\"codexy.repository-github-policy/v1\",\"repository\":\"eunsoogi/codexy\"}")?;

    let labeled = write_pr_state(
        temp.path(),
        "labeled.json",
        r#"{"number":216,"state":"OPEN","repository":"eunsoogi/codexy","labels":{"nodes":[{"name":"type/fix"},{"name":"area/workflow"}]},"repositoryLabels":{"nodes":[{"name":"type/fix"},{"name":"area/workflow"},{"name":"status/review"}]}}"#,
    )?;
    let good = Command::new(&script)
        .current_dir(&configured)
        .args([
            "--check-pr-labels",
            "--pr-state-file",
            labeled.to_str().ok_or("labeled state path")?,
        ])
        .output()?;
    assert!(
        good.status.success(),
        "installed guard should accept labeled PRs\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&good.stdout),
        String::from_utf8_lossy(&good.stderr)
    );

    let unlabeled = write_pr_state(
        temp.path(),
        "unlabeled.json",
        r#"{"number":216,"state":"OPEN","repository":"eunsoogi/codexy","labels":[],"repositoryLabels":{"nodes":[{"name":"type/fix"},{"name":"status/review"}]}}"#,
    )?;
    let bad = Command::new(&script)
        .current_dir(&configured)
        .args([
            "--check-pr-labels",
            "--pr-state-file",
            unlabeled.to_str().ok_or("unlabeled state path")?,
        ])
        .output()?;
    assert!(
        !bad.status.success(),
        "installed guard should reject unlabeled PRs when repository labels exist"
    );
    assert!(
        output_text(&bad).contains("PR labels missing label application evidence"),
        "unexpected output: {}",
        output_text(&bad)
    );

    let unconfigured = temp.path().join("unconfigured");
    std::fs::create_dir_all(unconfigured.join(".git"))?;
    let unconfigured = Command::new(&script)
        .current_dir(unconfigured)
        .args([
            "--check-pr-labels",
            "--pr-state-file",
            unlabeled.to_str().ok_or("unlabeled state path")?,
        ])
        .output()?;
    assert!(unconfigured.status.success(), "{}", output_text(&unconfigured));

    Ok(())
}

fn output_text(output: &std::process::Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn write_pr_state(
    dir: &std::path::Path,
    name: &str,
    json: &str,
) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    let path = dir.join(name);
    std::fs::write(&path, json)?;
    Ok(path)
}
