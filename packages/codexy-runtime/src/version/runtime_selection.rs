use std::{fs, path::Path};

use anyhow::{Context as _, Result};
use serde_json::Value;

pub(super) fn selected_tag(root: &Path) -> Result<String> {
    let path = root.join("plugins/codexy-devtools/runtime-release.json");
    let release: Value = serde_json::from_str(&fs::read_to_string(&path)?)
        .with_context(|| format!("invalid JSON in {}", path.display()))?;
    let tag = release
        .get("artifact")
        .and_then(|artifact| artifact.get("tag"))
        .and_then(Value::as_str)
        .context("runtime release artifact tag must be a string")?;
    let version = tag
        .strip_prefix('v')
        .context("runtime release artifact tag must start with v")?;
    super::require_semver(version)?;
    Ok(tag.to_owned())
}

pub(super) fn wrapper_version(root: &Path) -> Result<String> {
    let tag = selected_tag(root)?;
    let version = tag
        .strip_prefix('v')
        .context("runtime release artifact tag must start with v")?;
    Ok(version.to_owned())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::{selected_tag, wrapper_version};

    #[test]
    fn wrapper_version_follows_the_source_selected_runtime_pointer() -> anyhow::Result<()> {
        let temporary = tempfile::tempdir()?;
        let root = temporary.path();
        let release = root.join("plugins/codexy-devtools/runtime-release.json");
        fs::create_dir_all(release.parent().expect("release parent"))?;
        let current = fs::read_to_string(
            crate::paths::repository_root().join("plugins/codexy-devtools/runtime-release.json"),
        )?;
        fs::write(&release, current)?;
        assert_eq!(selected_tag(root)?, "v1.5.0");
        assert_eq!(wrapper_version(root)?, "1.5.0");
        fs::write(&release, r#"{"artifact":{"tag":"1.5.0"}}"#)?;
        assert!(wrapper_version(root).is_err());
        Ok(())
    }
}
