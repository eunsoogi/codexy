use std::path::Path;

use anyhow::{Context as _, Result, bail};
use serde_json::Value;

#[derive(Debug, Eq, PartialEq)]
pub enum VersionAdvanceAdmission {
    AlreadySelected,
    ActivatedSelection,
}

pub fn admit(target: &str) -> Result<VersionAdvanceAdmission> {
    super::require_semver(target)?;
    let root = crate::paths::repo_root()?;
    let manifest = super::load_json(&root.join(super::PLUGIN_MANIFEST))?;
    let current = super::string_field(&manifest, "version", "plugin manifest")?;
    super::require_semver(current)?;
    match semantic(target).cmp(&semantic(current)) {
        std::cmp::Ordering::Less => bail!(
            "version advance target {target} must not precede current plugin version {current}"
        ),
        std::cmp::Ordering::Equal => return Ok(VersionAdvanceAdmission::AlreadySelected),
        std::cmp::Ordering::Greater => {}
    }

    let publish = super::load_json(&root.join(super::PUBLISH_CONTRACT))?;
    let selected_bootstrap = nested_string(&publish, &["bootstrap", "selectedVersion"])?;
    if selected_bootstrap != target {
        bail!("version advance requires selected public bootstrap {target}");
    }
    if bootstrap_version(&root)? != target {
        bail!("version advance target does not match selected bootstrap source");
    }
    super::wrappers::check_version_at(&root, target)?;

    let release = super::load_json(&root.join("plugins/codexy/runtime-release.json"))?;
    if release["state"] != "candidate-proven" {
        bail!("version advance requires an activated candidate-proven runtime");
    }
    let selected_tag = nested_string(&publish, &["runtime", "selectedTag"])?;
    let release_tag = nested_string(&release, &["artifact", "tag"])?;
    if selected_tag != release_tag {
        bail!("version advance runtime selection does not match runtime-release.json");
    }
    crate::validation::run(&root.join("plugins/codexy"), crate::validation::Mode::All)?;
    Ok(VersionAdvanceAdmission::ActivatedSelection)
}

fn semantic(version: &str) -> (u64, u64, u64) {
    let mut parts = version
        .split('.')
        .map(|part| part.parse().unwrap_or(u64::MAX));
    (
        parts.next().unwrap(),
        parts.next().unwrap(),
        parts.next().unwrap(),
    )
}

fn nested_string<'a>(value: &'a Value, fields: &[&str]) -> Result<&'a str> {
    fields
        .iter()
        .try_fold(value, |current, field| {
            current.get(field).context("missing selected identity")
        })?
        .as_str()
        .filter(|value| !value.is_empty())
        .context("selected identity must be a non-empty string")
}

fn bootstrap_version(root: &Path) -> Result<String> {
    let source = std::fs::read_to_string(root.join("src/version/bootstrap.rs"))?;
    let prefix = "pub(super) const VERSION: &str = \"";
    let matches = source
        .lines()
        .filter_map(|line| line.strip_prefix(prefix)?.strip_suffix("\";"))
        .collect::<Vec<_>>();
    if let [version] = matches.as_slice() {
        super::require_semver(version)?;
        Ok((*version).to_owned())
    } else {
        bail!("selected bootstrap source must contain exactly one VERSION")
    }
}
