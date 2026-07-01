use std::process::Command;

#[allow(unused)]
mod support;

#[test]
fn installed_readiness_guard_validates_merge_bodies() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    support::copy_dir(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy"),
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
    Ok(())
}

fn output_text(output: &std::process::Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}
