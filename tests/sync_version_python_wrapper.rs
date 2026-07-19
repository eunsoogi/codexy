use std::{fs, process::Command};

use super::sync_version_cli::archive_repository;

#[test]
fn sync_version_cli_rejects_wrapper_version_prefix_decoys()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let repo = archive_repository(&temp, "version-prefix")?;
    for server in ["lsp", "codegraph"] {
        let wrapper = repo.join(format!("plugins/codexy/mcp/codexy-mcp-{server}"));
        let text = fs::read_to_string(&wrapper)?;
        fs::write(
            wrapper,
            text.replace("codexy-runtime-tools==1.2.1", "codexy-runtime-tools==1.2.10"),
        )?;
    }

    let output = Command::new(env!("CARGO_BIN_EXE_codexy-sync-version"))
        .arg("--check")
        .env("CODEXY_REPO_ROOT", &repo)
        .current_dir(&repo)
        .output()?;
    assert!(
        !output.status.success(),
        "wrapper version-prefix decoy unexpectedly passed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}
