use std::{path::Path, process::Command};

use anyhow::{Context as _, Result, bail};

use super::{PUBLISH_CONTRACT, load_json, require_semver, string_field, wrappers, write_json};

const PYPI_VERSION_ENDPOINT: &str = "https://pypi.org/pypi/getcodexy";

pub(super) fn version(publish: &serde_json::Value, label: &str) -> Result<String> {
    let version = string_field(publish, "bootstrapVersion", label)?;
    require_semver(version)?;
    Ok(version.to_owned())
}

pub(super) fn advance() -> Result<String> {
    let root = super::repo_root()?;
    advance_at(&root, check_pypi_availability)
}

fn advance_at<F>(root: &Path, check_available: F) -> Result<String>
where
    F: FnOnce(&str) -> Result<()>,
{
    let path = root.join(PUBLISH_CONTRACT);
    let mut publish = load_json(&path)?;
    let target = string_field(&publish, "version", &super::display_relative(&path))?.to_owned();
    require_semver(&target)?;
    let updates = wrappers::prepare_version_at(root, &target)?;
    check_available(&target)?;
    publish["bootstrapVersion"] = serde_json::Value::String(target.clone());
    write_json(&path, &publish)?;
    wrappers::write_updates(updates)?;
    Ok(format!("plugin bootstrap advanced to {target}"))
}

fn check_pypi_availability(version: &str) -> Result<()> {
    let mut command = Command::new("/usr/bin/curl");
    command.args([
        "--disable",
        "--fail",
        "--silent",
        "--show-error",
        "--connect-timeout",
        "5",
        "--max-time",
        "20",
        "--max-filesize",
        "1048576",
    ]);
    command.arg(format!("{PYPI_VERSION_ENDPOINT}/{version}/json"));
    if command
        .status()
        .with_context(|| format!("checking public PyPI availability for getcodexy {version}"))?
        .success()
    {
        Ok(())
    } else {
        bail!("getcodexy {version} is not available from public PyPI")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    const LSP: &str = "plugins/codexy/mcp/codexy-mcp-lsp";
    const CODEGRAPH: &str = "plugins/codexy/mcp/codexy-mcp-codegraph";

    #[test]
    fn advancement_is_atomic_after_an_injected_publication_check() -> Result<()> {
        let temp = tempfile::tempdir()?;
        write_fixture(temp.path())?;
        let tracked = tracked_contents(temp.path())?;
        assert!(advance_at(temp.path(), |_| bail!("not public")).is_err());
        assert_eq!(
            tracked_contents(temp.path())?,
            tracked,
            "unavailable package mutated tracked files"
        );

        assert_eq!(
            advance_at(temp.path(), |_| Ok(()))?,
            "plugin bootstrap advanced to 9.9.9"
        );
        let contract: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(temp.path().join(PUBLISH_CONTRACT))?)?;
        assert_eq!(contract["bootstrapVersion"], "9.9.9");
        for wrapper in [LSP, CODEGRAPH] {
            assert!(fs::read_to_string(temp.path().join(wrapper))?.contains("getcodexy==9.9.9"));
        }
        Ok(())
    }

    fn write_fixture(root: &Path) -> Result<()> {
        let contract = root.join(PUBLISH_CONTRACT);
        fs::create_dir_all(contract.parent().context("contract parent")?)?;
        fs::write(
            contract,
            "{\"version\":\"9.9.9\",\"bootstrapVersion\":\"1.2.2\"}\n",
        )?;
        for (path, server) in [(LSP, "lsp"), (CODEGRAPH, "codegraph")] {
            let wrapper = root.join(path);
            fs::create_dir_all(wrapper.parent().context("wrapper parent")?)?;
            fs::write(
                wrapper,
                format!(
                    "#!/bin/sh\nexec uvx --from getcodexy==1.2.2 codexy-mcp-runtime {server} --plugin-root \"$0\" \"$@\"\n"
                ),
            )?;
        }
        Ok(())
    }

    fn tracked_contents(root: &Path) -> Result<Vec<Vec<u8>>> {
        [PUBLISH_CONTRACT, LSP, CODEGRAPH]
            .into_iter()
            .map(|path| Ok(fs::read(root.join(path))?))
            .collect()
    }
}
