use std::fs::File;
use std::process::Command;

#[test]
fn readiness_guard_treats_globs_as_literal_merge_message_text()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    File::create(temp.path().join("Fixes #206"))?;

    let output = Command::new(readiness_guard())
        .current_dir(temp.path())
        .args([
            "--check-merge-message",
            "--expected-pr",
            "204",
            "--expected-issue",
            "206",
            "--merge-message",
            "fix(workflow): x (#204)\n\n*\n",
        ])
        .output()?;

    assert!(
        !output.status.success(),
        "literal glob line must not expand to a matching closing reference\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output_text(&output).contains("final closing line must be exactly: Fixes #206"),
        "unexpected output: {}",
        output_text(&output)
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

fn readiness_guard() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("plugins/codexy/hooks/codexy-readiness-guard.sh")
}
