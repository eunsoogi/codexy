use std::{fs, process::Command};

use super::sync_version_cli::archive_repository;

fn run_check(repo: &std::path::Path) -> Result<std::process::Output, Box<dyn std::error::Error>> {
    Ok(Command::new(env!("CARGO_BIN_EXE_codexy-sync-version"))
        .arg("--check")
        .env("CODEXY_REPO_ROOT", repo)
        .current_dir(repo)
        .output()?)
}

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
            text.replace("eunsoogi-codexy==1.2.1", "eunsoogi-codexy==1.2.10"),
        )?;
    }

    let output = run_check(&repo)?;
    assert!(
        !output.status.success(),
        "wrapper version-prefix decoy unexpectedly passed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn sync_version_cli_rejects_pin_on_a_different_command()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let repo = archive_repository(&temp, "different-command")?;
    for server in ["lsp", "codegraph"] {
        let wrapper = repo.join(format!("plugins/codexy/mcp/codexy-mcp-{server}"));
        let text = fs::read_to_string(&wrapper)?;
        fs::write(
            wrapper,
            format!(
                "printf '%s\\n' \"eunsoogi-codexy==1.2.1\"\n{}",
                text.replace("eunsoogi-codexy==1.2.1", "eunsoogi-codexy==1.2.10")
            ),
        )?;
    }

    assert!(!run_check(&repo)?.status.success());
    Ok(())
}

#[test]
fn sync_version_cli_rejects_semicolon_comment_pin_decoy()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let repo = archive_repository(&temp, "semicolon-comment")?;
    for server in ["lsp", "codegraph"] {
        let wrapper = repo.join(format!("plugins/codexy/mcp/codexy-mcp-{server}"));
        let text = fs::read_to_string(&wrapper)?;
        fs::write(
            wrapper,
            format!(
                "true;# \"eunsoogi-codexy==1.2.1\"\n{}",
                text.replace("eunsoogi-codexy==1.2.1", "eunsoogi-codexy==1.2.10")
            ),
        )?;
    }

    assert!(!run_check(&repo)?.status.success());
    Ok(())
}

#[test]
fn sync_version_cli_ignores_comment_pins_when_updating()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let repo = archive_repository(&temp, "comment-pin-update")?;
    for server in ["lsp", "codegraph"] {
        let wrapper = repo.join(format!("plugins/codexy/mcp/codexy-mcp-{server}"));
        let text = fs::read_to_string(&wrapper)?;
        fs::write(
            wrapper,
            format!("# keep eunsoogi-codexy==1.2.1 as historical context\n{text}"),
        )?;
    }

    let output = Command::new(env!("CARGO_BIN_EXE_codexy-sync-version"))
        .args(["--version", "9.9.9"])
        .env("CODEXY_REPO_ROOT", &repo)
        .current_dir(&repo)
        .output()?;
    assert!(
        output.status.success(),
        "update failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    for server in ["lsp", "codegraph"] {
        let wrapper = repo.join(format!("plugins/codexy/mcp/codexy-mcp-{server}"));
        let text = fs::read_to_string(wrapper)?;
        assert!(text.starts_with("# keep eunsoogi-codexy==1.2.1"));
        let expected = format!(
            "  --from \"eunsoogi-codexy==9.9.9\" codexy-mcp-runtime {server} \\"
        );
        assert_eq!(
            text.lines().find(|line| line.starts_with("  --from ")),
            Some(expected.as_str())
        );
    }
    Ok(())
}
