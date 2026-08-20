use std::{collections::BTreeMap, fs, path::Path, process::Command};

use serde_json::Value;

use super::archive_repository;

#[test]
fn already_selected_version_sync_preserves_runtime_pointers()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let repo = archive_repository(&temp)?;
    let manifest: Value = serde_json::from_slice(&fs::read(
        repo.join("plugins/codexy-devtools/.codex-plugin/plugin.json"),
    )?)?;
    let selected = manifest["version"].as_str().ok_or("selected version")?;
    sync_selected(&repo, selected)?;
    let preserved = [
        (
            "plugins/codexy-devtools/runtime-release.json",
            "{\"runtime\":\"immutable\"}\n",
        ),
        (
            "plugins/codexy-devtools/mcp/codexy-mcp-lsp",
            "#!/bin/sh\necho pinned\n",
        ),
        (
            "plugins/codexy-devtools/mcp/codexy-mcp-codegraph",
            "#!/bin/sh\necho pinned\n",
        ),
    ];
    for (relative, contents) in preserved {
        let path = repo.join(relative);
        fs::create_dir_all(path.parent().ok_or("fixture parent")?)?;
        fs::write(path, contents)?;
    }
    let mut before = preserved
        .iter()
        .map(|(relative, _)| Ok((relative.to_string(), fs::read(repo.join(relative))?)))
        .collect::<Result<BTreeMap<_, _>, std::io::Error>>()?;
    let bootstrap = "packages/getcodexy/pyproject.toml";
    before.insert(bootstrap.into(), fs::read(repo.join(bootstrap))?);
    sync_selected(&repo, selected)?;
    for (relative, expected) in before {
        assert_eq!(
            fs::read(repo.join(&relative))?,
            expected,
            "ordinary version sync changed {relative}"
        );
    }
    Ok(())
}

fn sync_selected(root: &Path, selected: &str) -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new(env!("CARGO_BIN_EXE_codexy-sync-version"))
        .args(["--version", selected])
        .env("CODEXY_REPO_ROOT", root)
        .current_dir(root)
        .output()?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "selected fixture normalization failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into())
    }
}
