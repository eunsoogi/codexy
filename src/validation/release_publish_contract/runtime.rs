use std::path::Path;

use anyhow::{Context as _, Result, bail};
use serde_json::Value;

use crate::paths::display_relative;
use crate::validation::json_array_strings;

use super::require_exact;

const STAGING_WORKFLOW: &str = ".github/workflows/runtime-candidate.yml";
const ACTIVATION_WORKFLOW: &str = ".github/workflows/runtime-activation.yml";
const FINAL_PUBLISHER_WORKFLOW: &str = ".github/workflows/publish-version-release.yml";
const RETENTION_DAYS: i64 = 14;

pub(super) fn check(contract: &Value, path: &Path) -> Result<()> {
    let runtime = contract
        .get("runtime")
        .and_then(Value::as_object)
        .with_context(|| format!("{} runtime must be an object", display_relative(path)))?;
    let mut keys = runtime.keys().map(String::as_str).collect::<Vec<_>>();
    keys.sort_unstable();
    let mut expected = [
        "activationWorkflow",
        "artifactRetentionDays",
        "finalPublisherWorkflow",
        "platforms",
        "selectedTag",
        "stagingWorkflow",
    ];
    expected.sort_unstable();
    if keys != expected {
        bail!(
            "{} runtime must describe authenticated staging and version-only publication",
            display_relative(path)
        );
    }
    let selected_tag = runtime
        .get("selectedTag")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let version = selected_tag.strip_prefix('v').unwrap_or_default();
    if version.split('.').count() != 3
        || !version.split('.').all(|component| {
            !component.is_empty() && component.bytes().all(|byte| byte.is_ascii_digit())
        })
    {
        bail!(
            "{} runtime.selectedTag must be a version-only vMAJOR.MINOR.PATCH tag",
            display_relative(path)
        );
    }
    require_exact(
        runtime.get("stagingWorkflow"),
        "runtime.stagingWorkflow",
        path,
        STAGING_WORKFLOW,
    )?;
    require_exact(
        runtime.get("activationWorkflow"),
        "runtime.activationWorkflow",
        path,
        ACTIVATION_WORKFLOW,
    )?;
    require_exact(
        runtime.get("finalPublisherWorkflow"),
        "runtime.finalPublisherWorkflow",
        path,
        FINAL_PUBLISHER_WORKFLOW,
    )?;
    if runtime.get("artifactRetentionDays").and_then(Value::as_i64) != Some(RETENTION_DAYS) {
        bail!(
            "{} runtime.artifactRetentionDays must be {RETENTION_DAYS}",
            display_relative(path)
        );
    }
    Ok(())
}

pub(super) fn release_archive_platforms(contract: &Value, path: &Path) -> Result<Vec<String>> {
    let archive = contract
        .get("releaseArchive")
        .and_then(Value::as_object)
        .with_context(|| {
            format!(
                "{} releaseArchive must be an object",
                display_relative(path)
            )
        })?;
    json_array_strings(archive.get("platforms")).with_context(|| {
        format!(
            "{} releaseArchive.platforms must be an array",
            display_relative(path)
        )
    })
}
