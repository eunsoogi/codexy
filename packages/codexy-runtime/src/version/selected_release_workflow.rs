use std::{fs, path::Path};

use anyhow::{Context as _, Result, bail};
use regex::Regex;
use serde_yaml::Value;

use crate::paths::display_relative;

const WORKFLOW: &str = ".github/workflows/python-package.yml";
const HELPER: &str = "scripts/verify_public_marketplace_bundle.py";
const SUPPORT: &str = "scripts/public_marketplace_bundle_support.py";
const JOB: &str = "github-activation-windows";
const STEP: &str = "Run native component lifecycle tests";

pub(super) fn check(root: &Path) -> Result<()> {
    let path = root.join(WORKFLOW);
    let text = fs::read_to_string(&path)
        .with_context(|| format!("missing required file: {}", display_relative(&path)))?;
    let workflow: Value = serde_yaml::from_str(&text)
        .with_context(|| format!("invalid YAML in {}", display_relative(&path)))?;
    let lifecycle = lifecycle_run(&workflow, &path)?;
    check_lifecycle(lifecycle, &path)?;
    check_helper(root, &path, &text)?;
    check_support(root)
}

fn lifecycle_run<'a>(workflow: &'a Value, path: &Path) -> Result<&'a str> {
    workflow
        .get("jobs")
        .and_then(Value::as_mapping)
        .and_then(|jobs| jobs.get(Value::String(JOB.to_owned())))
        .and_then(|job| job.get("steps"))
        .and_then(Value::as_sequence)
        .and_then(|steps| {
            steps
                .iter()
                .find(|step| step.get("name") == Some(&Value::String(STEP.to_owned())))
        })
        .and_then(|step| step.get("run"))
        .and_then(Value::as_str)
        .with_context(|| format!("{} lifecycle step is missing", display_relative(path)))
}

fn check_lifecycle(run: &str, path: &Path) -> Result<()> {
    require_ordered(
        run,
        &[
            "$verified = python scripts/verify_public_marketplace_bundle.py --output-dir $bundleRoot",
            "if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }",
            "$release = $verified | ConvertFrom-Json",
            "$binary = $release.watcher_binary",
            "$env:CODEXY_TEST_WATCHER_BINARY = $binary",
            "\"CODEXY_TEST_WATCHER_BINARY=$binary\" | Out-File -FilePath $env:GITHUB_ENV -Encoding utf8 -Append",
        ],
        path,
    )?;
    for (prefix, expected) in [
        (
            "$verified =",
            "$verified = python scripts/verify_public_marketplace_bundle.py --output-dir $bundleRoot",
        ),
        ("$release =", "$release = $verified | ConvertFrom-Json"),
        ("$binary =", "$binary = $release.watcher_binary"),
        (
            "$env:CODEXY_TEST_WATCHER_BINARY =",
            "$env:CODEXY_TEST_WATCHER_BINARY = $binary",
        ),
        (
            "\"CODEXY_TEST_WATCHER_BINARY=",
            "\"CODEXY_TEST_WATCHER_BINARY=$binary\" | Out-File -FilePath $env:GITHUB_ENV -Encoding utf8 -Append",
        ),
    ] {
        let assignments = run
            .lines()
            .map(str::trim)
            .filter(|line| line.starts_with(prefix))
            .collect::<Vec<_>>();
        if assignments.as_slice() != [expected] {
            bail!(
                "{} must bind the tested binary to the verified helper result",
                display_relative(path)
            );
        }
    }
    let release_version =
        Regex::new(r"(?i)(?:^|[^[:alnum:]])v?[0-9]+\.[0-9]+\.[0-9]+(?:$|[^[:alnum:]])")?;
    for forbidden in [
        "gh release",
        "Invoke-WebRequest",
        "Invoke-RestMethod",
        "Start-BitsTransfer",
        "System.Net.WebClient",
        "releases/download/",
    ] {
        if run.contains(forbidden) {
            bail!(
                "{} lifecycle step must not perform a direct public release operation: {forbidden:?}",
                display_relative(path)
            );
        }
    }
    if release_version.is_match(run) {
        bail!(
            "{} lifecycle step must not contain a literal release version",
            display_relative(path)
        );
    }
    Ok(())
}

fn require_ordered(run: &str, required: &[&str], path: &Path) -> Result<()> {
    let mut offset = 0;
    for needle in required {
        let found = run[offset..].find(needle).with_context(|| {
            format!(
                "{} lifecycle step is missing {needle:?}",
                display_relative(path)
            )
        })?;
        offset += found + needle.len();
    }
    Ok(())
}

fn check_helper(root: &Path, workflow: &Path, workflow_text: &str) -> Result<()> {
    let path = root.join(HELPER);
    let text = fs::read_to_string(&path)
        .with_context(|| format!("missing required file: {}", display_relative(&path)))?;
    for required in [
        "from public_marketplace_bundle_support import",
        "CONTRACT,",
        "current_tag, _ = contract_tag(current_contract",
        "release_tag = current_tag",
        "version = current_tag.removeprefix(\"v\")",
        "\"PRIOR_PUBLIC_VERSION\"",
        "\"BASE_SHA\"",
        "\"gh\",",
        "\"release\",",
        "\"download\",",
        "runtime-release-receipt.json",
    ] {
        if !text.contains(required) {
            bail!(
                "{} must derive and verify the selected public release; missing {required:?}",
                display_relative(&path)
            );
        }
    }
    reject_literal_version(&text, &path)?;
    for required in [HELPER, "BASE_SHA", "PRIOR_PUBLIC_VERSION"] {
        if !workflow_text.contains(required) {
            bail!(
                "{} must route the lifecycle through the trusted helper; missing {required:?}",
                display_relative(workflow)
            );
        }
    }
    Ok(())
}

fn check_support(root: &Path) -> Result<()> {
    let path = root.join(SUPPORT);
    let text = fs::read_to_string(&path)
        .with_context(|| format!("missing required file: {}", display_relative(&path)))?;
    for required in [
        "CONTRACT = Path(\".agents/plugins/release-publish-contract.json\")",
        "tarfile.open",
        "archive.extract",
        "runtime-release-receipt/v2",
        "manifestSha256",
    ] {
        if !text.contains(required) {
            bail!(
                "{} must provide public release verification support; missing {required:?}",
                display_relative(&path)
            );
        }
    }
    reject_literal_version(&text, &path)
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
