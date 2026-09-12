use std::{fs, path::Path};

use anyhow::{Context as _, Result, bail};
use serde_json::Value;

use crate::paths::display_relative;

#[path = "selected_release_workflow.rs"]
mod workflow;

const CONTRACT: &str = ".agents/plugins/release-publish-contract.json";
const BOOTSTRAP: &str = "packages/codexy-runtime/src/version/bootstrap.rs";
const RUNTIME_RELEASE: &str = "plugins/codexy-devtools/runtime-release.json";
const REPOSITORY: &str = "https://github.com/eunsoogi/codexy";

/// Checks the finite production projections of the selected public release.
pub(crate) fn check(root: &Path, expected: &str) -> Result<()> {
    super::require_semver(expected)?;
    let contract_path = root.join(CONTRACT);
    let contract = read_json(&contract_path)?;
    let expected_tag = format!("v{expected}");
    let selected_version =
        nested_string(&contract, &["bootstrap", "selectedVersion"], &contract_path)?;
    super::require_matching_version(
        selected_version,
        "release publish contract bootstrap.selectedVersion",
        expected,
        "selected release",
    )?;
    let selected_tag = nested_string(&contract, &["runtime", "selectedTag"], &contract_path)?;
    if selected_tag != expected_tag {
        bail!(
            "{} runtime.selectedTag must be {expected_tag:?}, got {selected_tag:?}",
            display_relative(&contract_path)
        );
    }

    let bootstrap_path = root.join(BOOTSTRAP);
    let bootstrap_version = bootstrap_version(&bootstrap_path)?;
    super::require_matching_version(
        &bootstrap_version,
        &display_relative(&bootstrap_path),
        expected,
        "selected release",
    )?;

    let release_path = root.join(RUNTIME_RELEASE);
    let release = read_json(&release_path)?;
    let artifact = release
        .get("artifact")
        .and_then(Value::as_object)
        .with_context(|| {
            format!(
                "{} artifact must be an object",
                display_relative(&release_path)
            )
        })?;
    let artifact_tag = artifact
        .get("tag")
        .and_then(Value::as_str)
        .context("runtime release artifact tag must be a string")?;
    let runtime_version = artifact_tag
        .strip_prefix('v')
        .context("runtime release artifact tag must start with v")?;
    super::require_semver(runtime_version)?;
    if semantic(runtime_version) > semantic(expected) {
        bail!(
            "{} artifact.tag must not be newer than release contract runtime.selectedTag: expected at most {expected_tag:?}, got {artifact_tag:?}",
            display_relative(&release_path)
        );
    }
    let expected_url =
        format!("{REPOSITORY}/releases/download/{artifact_tag}/codexy-runtime-package.tar.gz");
    let artifact_url = artifact
        .get("url")
        .and_then(Value::as_str)
        .context("runtime release artifact URL must be a string")?;
    if artifact_url != expected_url {
        bail!(
            "{} artifact.url must be {expected_url:?}, got {artifact_url:?}",
            display_relative(&release_path)
        );
    }

    super::wrappers::check_version_at(root, runtime_version)?;
    workflow::check(root)
}

fn semantic(version: &str) -> (u64, u64, u64) {
    let mut parts = version
        .split('.')
        .map(|part| part.parse().unwrap_or(u64::MAX));
    (
        parts.next().unwrap_or(u64::MAX),
        parts.next().unwrap_or(u64::MAX),
        parts.next().unwrap_or(u64::MAX),
    )
}

fn read_json(path: &Path) -> Result<Value> {
    let text = fs::read_to_string(path)
        .with_context(|| format!("missing required file: {}", display_relative(path)))?;
    serde_json::from_str(&text)
        .with_context(|| format!("invalid JSON in {}", display_relative(path)))
}

fn nested_string<'a>(value: &'a Value, fields: &[&str], path: &Path) -> Result<&'a str> {
    fields
        .iter()
        .try_fold(value, |current, field| {
            current
                .get(field)
                .with_context(|| format!("{} missing {field}", display_relative(path)))
        })?
        .as_str()
        .filter(|value| !value.is_empty())
        .with_context(|| {
            format!(
                "{} selected release identity must be a string",
                display_relative(path)
            )
        })
}

fn bootstrap_version(path: &Path) -> Result<String> {
    let text = fs::read_to_string(path)
        .with_context(|| format!("missing required file: {}", display_relative(path)))?;
    let prefix = "pub(super) const VERSION: &str = \"";
    let versions = text
        .lines()
        .filter_map(|line| line.strip_prefix(prefix)?.strip_suffix("\";"))
        .collect::<Vec<_>>();
    let [version] = versions.as_slice() else {
        bail!(
            "{} must contain exactly one selected VERSION",
            display_relative(path)
        );
    };
    super::require_semver(version)?;
    Ok((*version).to_owned())
}
