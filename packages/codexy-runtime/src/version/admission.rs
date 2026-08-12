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
    let plugin = root.join("plugins/codexy-devtools");
    let prior_release = super::load_json(&plugin.join("runtime-release.json"))?;
    let prior_tag = nested_string(&prior_release, &["artifact", "tag"])?;
    let prior_version = prior_tag
        .strip_prefix('v')
        .context("prior public runtime tag must start with v")?;
    super::require_semver(prior_version)?;
    super::wrappers::check_version_at(&root, prior_version)?;
    if plugin.join("runtime-candidate.json").exists() {
        bail!("version advance requires no private runtime candidate in the source plugin");
    }
    let record = super::load_json(&root.join(".agents/plugins/runtime-activation.json"))?;
    let candidate = record
        .get("candidate")
        .and_then(Value::as_object)
        .context("version advance activation record must contain candidate")?;
    let artifact = candidate
        .get("artifact")
        .and_then(Value::as_object)
        .context("version advance activation record candidate must contain artifact")?;
    for field in ["stagingRunId", "stagingRunAttempt"] {
        if !artifact
            .get(field)
            .is_some_and(|value| value.is_u64() && value.as_u64() > Some(0))
        {
            bail!("version advance activation record {field} must be a positive integer");
        }
    }
    let selected_tag = nested_string(&publish, &["runtime", "selectedTag"])?;
    if selected_tag != format!("v{target}") {
        bail!("version advance runtime selection does not match the public bootstrap tag");
    }
    crate::validation::run(&plugin, crate::validation::Mode::All)?;
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
    let source = std::fs::read_to_string(super::runtime_package_path(
        root,
        "src/version/bootstrap.rs",
    ))?;
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
