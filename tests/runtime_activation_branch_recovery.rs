use std::{fs, os::unix::fs::PermissionsExt as _, path::{Path, PathBuf}, process::Command};

const AUTHORIZED: &str = ".agents/plugins/release-publish-contract.json\nplugins/codexy/mcp/codexy-mcp-codegraph\nplugins/codexy/mcp/codexy-mcp-lsp\nplugins/codexy/runtime-candidate.json\nplugins/codexy/runtime-release.json\nsrc/version/bootstrap.rs\n";

#[test]
fn existing_activation_branch_rejects_unrelated_changes_and_closed_pull_requests()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = Fixture::new()?;
    assert!(fixture.run(AUTHORIZED, "OPEN")?.status.success());
    assert!(!fixture.run(&format!("docs/notes.md\n{AUTHORIZED}"), "OPEN")?.status.success());
    assert!(!fixture.run(AUTHORIZED, "CLOSED")?.status.success());
    Ok(())
}

struct Fixture { _temp: tempfile::TempDir, bin: PathBuf }

impl Fixture {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let bin = temp.path().join("bin");
        fs::create_dir_all(&bin)?;
        for name in ["git", "gh"] { fake_command(&bin.join(name))?; }
        Ok(Self { _temp: temp, bin })
    }

    fn run(&self, diff: &str, pr_state: &str) -> Result<std::process::Output, Box<dyn std::error::Error>> {
        Ok(Command::new(Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/verify-runtime-activation-branch"))
            .arg("codexy/runtime-activation-runtime-candidate-test")
            .env("PATH", format!("{}:{}", self.bin.display(), std::env::var("PATH")?))
            .env("FAKE_DIFF", diff)
            .env("FAKE_PR_STATE", pr_state)
            .output()?)
    }
}

fn fake_command(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    fs::write(path, "#!/bin/sh\nset -eu\ncase \"$(basename \"$0\")\" in git) printf '%s' \"$FAKE_DIFF\" ;; gh) printf '%s\\n' \"$FAKE_PR_STATE\" ;; esac\n")?;
    let mut permissions = fs::metadata(path)?.permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions)?;
    Ok(())
}
