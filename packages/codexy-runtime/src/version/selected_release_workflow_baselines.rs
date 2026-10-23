//! Checks that the Windows workflow keeps public and legacy bundle sources distinct.

use std::{fs, path::Path};

use anyhow::{Context as _, Result, bail};
use regex::Regex;
use serde_yaml::Value;

use crate::paths::display_relative;

const WORKFLOW: &str = ".github/workflows/python-package.yml";
const BASELINES: &str = ".github/scripts/verify-public-marketplace-baselines.ps1";
const JOB: &str = "github-activation-windows";
const RUNTIME_STEP: &str = "Verify candidate wheel runtime handoff on native Windows";

pub(super) fn check(root: &Path) -> Result<()> {
    let path = root.join(WORKFLOW);
    let text = fs::read_to_string(&path)
        .with_context(|| format!("missing required file: {}", display_relative(&path)))?;
    let workflow: Value = serde_yaml::from_str(&text)
        .with_context(|| format!("invalid YAML in {}", display_relative(&path)))?;
    check_candidate_runtime(&workflow, &path)?;
    check_baseline_script(root)
}

fn candidate_runtime_run<'a>(workflow: &'a Value, path: &Path) -> Result<&'a str> {
    workflow
        .get("jobs")
        .and_then(Value::as_mapping)
        .and_then(|jobs| jobs.get(Value::String(JOB.to_owned())))
        .and_then(|job| job.get("steps"))
        .and_then(Value::as_sequence)
        .and_then(|steps| {
            steps
                .iter()
                .find(|step| step.get("name") == Some(&Value::String(RUNTIME_STEP.to_owned())))
        })
        .and_then(|step| step.get("run"))
        .and_then(Value::as_str)
        .with_context(|| format!("{} runtime handoff step is missing", display_relative(path)))
}

fn check_candidate_runtime(workflow: &Value, path: &Path) -> Result<()> {
    // Candidate smoke tests must consume the extracted root whose digest the verifier logged.
    let run = candidate_runtime_run(workflow, path)?;
    require_ordered(
        run,
        &[
            "$runtimeDir = Join-Path $env:CODEXY_PUBLIC_BUNDLE_ROOT \"plugins/codexy/runtime\"",
            "if (-not (Test-Path -LiteralPath $runtimeDir -PathType Container))",
        ],
        path,
    )
}

fn check_baseline_script(root: &Path) -> Result<()> {
    let path = root.join(BASELINES);
    let text = fs::read_to_string(&path)
        .with_context(|| format!("missing required file: {}", display_relative(&path)))?;
    for required in [
        "python scripts/verify_public_marketplace_bundle.py --output-dir $bundleRoot",
        "$release = $verified | ConvertFrom-Json",
        "$release.extracted_root",
        "--legacy-upgrade --output-dir $legacyRoot",
        "$legacyRelease.legacy_watcher_binary",
        "$legacyRelease.legacy_upgrade_fixture",
        "$legacyRelease.extracted_root",
        "CODEXY_PUBLIC_BUNDLE_ROOT=",
        "CODEXY_TEST_LEGACY_MARKETPLACE_ROOT=",
        "$env:GITHUB_ENV",
    ] {
        if !text.contains(required) {
            bail!(
                "{} must bind selected and legacy fixture paths to verified output; missing {required:?}",
                display_relative(&path)
            );
        }
    }
    for (prefix, expected) in [
        (
            "$env:CODEXY_PUBLIC_BUNDLE_ROOT =",
            "$env:CODEXY_PUBLIC_BUNDLE_ROOT = [string]$release.extracted_root",
        ),
        (
            "$env:CODEXY_TEST_LEGACY_MARKETPLACE_ROOT =",
            "$env:CODEXY_TEST_LEGACY_MARKETPLACE_ROOT = [string]$legacyRelease.extracted_root",
        ),
        (
            "\"CODEXY_PUBLIC_BUNDLE_ROOT=",
            "\"CODEXY_PUBLIC_BUNDLE_ROOT=$($release.extracted_root)\" | Out-File -FilePath $env:GITHUB_ENV -Encoding utf8 -Append",
        ),
        (
            "\"CODEXY_TEST_LEGACY_MARKETPLACE_ROOT=",
            "\"CODEXY_TEST_LEGACY_MARKETPLACE_ROOT=$($legacyRelease.extracted_root)\" | Out-File -FilePath $env:GITHUB_ENV -Encoding utf8 -Append",
        ),
    ] {
        let assignments = text
            .lines()
            .map(str::trim)
            .filter(|line| line.starts_with(prefix))
            .collect::<Vec<_>>();
        if assignments.as_slice() != [expected] {
            bail!(
                "{} must bind the legacy fixture to verified bundle output",
                display_relative(&path)
            );
        }
    }
    for forbidden in [
        "gh release",
        "Invoke-WebRequest",
        "Invoke-RestMethod",
        "Start-BitsTransfer",
        "System.Net.WebClient",
        "releases/download/",
    ] {
        if text.contains(forbidden) {
            bail!(
                "{} must keep release downloads inside the verifier: {forbidden:?}",
                display_relative(&path)
            );
        }
    }
    reject_literal_version(&text, &path)
}

fn require_ordered(run: &str, required: &[&str], path: &Path) -> Result<()> {
    let mut offset = 0;
    for needle in required {
        let found = run[offset..].find(needle).with_context(|| {
            format!(
                "{} runtime handoff step is missing {needle:?}",
                display_relative(path)
            )
        })?;
        offset += found + needle.len();
    }
    Ok(())
}

fn reject_literal_version(text: &str, path: &Path) -> Result<()> {
    let assignment = Regex::new(r#"(?m)^\s*version\s*=\s*[\"'][0-9]"#)?;
    if assignment.is_match(text) {
        bail!(
            "{} must not hard-code a public release version",
            display_relative(path)
        );
    }
    Ok(())
}
