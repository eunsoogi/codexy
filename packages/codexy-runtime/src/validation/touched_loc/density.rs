use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context as _, Result, bail};
use serde_json::Value;

mod analysis;
mod spans;
use analysis::reason;
use spans::{language, visible_lines};
/// Returns a diagnostic only for a changed, maintained source line whose
/// syntax packs several executable statements or fields into one construct.
/// Line width is intentionally not an input: long identifiers, URLs, and
/// protocol literals are not evidence of LOC-compliance compression.
pub(super) fn error(path: &Path, text: &str) -> Option<String> {
    if source_disposition(path, text) != Disposition::Maintained {
        return None;
    }
    let language = language(path, text);
    text.lines()
        .zip(visible_lines(language, text))
        .enumerate()
        .find_map(|(index, (_, visible))| {
            visible
                .as_deref()
                .and_then(|line| reason(language, line))
                .map(|reason| {
                    format!(
                        "{}:{} contains {reason}; expand or extract the maintained source instead of compressing it",
                        path.display(),
                        index + 1,
                    )
                })
        })
}

pub(super) fn is_governed_path(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|extension| extension.to_str()),
        Some(
            "rs" | "py"
                | "sh"
                | "ps1"
                | "js"
                | "ts"
                | "tsx"
                | "jsx"
                | "md"
                | "json"
                | "toml"
                | "yml"
                | "yaml"
        )
    ) || path.starts_with("plugins/codexy/hooks/")
        || path.starts_with("plugins/codexy-github/hooks/")
        || path.starts_with("scripts/")
}

/// Classifies each structural candidate and each wide-line audit input without
/// using width as a failing policy. The report is stable and source-addressable
/// so a review can distinguish an exact fixture from maintained source.
pub(super) fn inventory_at(root: &Path) -> Result<Vec<String>> {
    let output = Command::new("git")
        .args(["ls-files", "--cached", "--others", "--exclude-standard"])
        .current_dir(root)
        .output()
        .context("listing source files for density inventory")?;
    if !output.status.success() {
        bail!(
            "git ls-files for density inventory failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    let mut records = Vec::new();
    for path in String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(PathBuf::from)
    {
        if !is_governed_path(&path) || !root.join(&path).is_file() {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(root.join(&path)) else {
            continue;
        };
        let disposition = source_disposition(&path, &text);
        let language = language(&path, &text);
        for (index, (_line, visible)) in
            text.lines().zip(visible_lines(language, &text)).enumerate()
        {
            let reason = visible.as_deref().and_then(|line| reason(language, line));
            let structural = reason.is_some();
            if !structural {
                continue;
            }
            let classification = match disposition {
                Disposition::Maintained if structural => "confirmed-density-defect",
                Disposition::Maintained => "maintained-readable",
                Disposition::ExactFixture => "exact-fixture",
                Disposition::Generated => "generated",
                Disposition::Vendor => "vendor",
            };
            records.push(format!(
                "{}:{}\t{classification}\taudit-input={}",
                path.display(),
                index + 1,
                if structural {
                    "structural-density"
                } else {
                    "structural-density"
                },
            ));
        }
    }
    records.sort();
    Ok(records)
}

#[derive(Debug, Eq, PartialEq)]
pub(super) enum Disposition {
    Maintained,
    ExactFixture,
    Generated,
    Vendor,
}

pub(super) fn disposition(path: &Path) -> Disposition {
    let path = path.to_string_lossy();
    let lower = path.to_ascii_lowercase();
    if lower.starts_with("vendor/") || lower.contains("/vendor/") || lower.contains("node_modules/")
    {
        Disposition::Vendor
    } else if lower.starts_with(".codex/")
        || lower.starts_with("target/")
        || lower.contains("/target/")
    {
        Disposition::Generated
    } else if matches!(
        lower.as_str(),
        "packages/codexy-runtime/cargo.lock"
            | "packages/getcodexy/src/codexy_runtime_tools/component-manifest.json"
            | "plugins/codexy/runtime-activation.json"
            | "plugins/codexy/runtime-release.json"
    ) {
        Disposition::Generated
    } else {
        Disposition::Maintained
    }
}

fn source_disposition(path: &Path, text: &str) -> Disposition {
    let base = disposition(path);
    if base != Disposition::Maintained {
        return base;
    }
    if manifested_exact_fixture(path, text) {
        return Disposition::ExactFixture;
    }
    let json = serde_json::from_str::<Value>(text).ok();
    if json.as_ref().is_some_and(exact_json_fixture) {
        Disposition::ExactFixture
    } else {
        Disposition::Maintained
    }
}

fn manifested_exact_fixture(path: &Path, text: &str) -> bool {
    let Some(marker) = text
        .lines()
        .next()
        .and_then(|line| line.strip_prefix("// codexy-exact-fixture-file: "))
    else {
        return false;
    };
    let Ok(manifest) =
        serde_json::from_str::<Value>(include_str!("density/exact_fixture_manifest.json"))
    else {
        return false;
    };
    manifest.get("schema").and_then(Value::as_str) == Some("codexy.exact-fixture-manifest.v1")
        && manifest
            .get("fixtures")
            .and_then(Value::as_array)
            .is_some_and(|fixtures| {
                fixtures.iter().any(|fixture| {
                    fixture.get("path").and_then(Value::as_str) == path.to_str()
                        && fixture.get("marker").and_then(Value::as_str) == Some(marker)
                })
            })
}

fn exact_json_fixture(value: &Value) -> bool {
    let Some(object) = value.as_object() else {
        return false;
    };
    let routing = routing_corpus(object)
        || routing_results(object)
        || object
            .get("$schema")
            .and_then(Value::as_str)
            .is_some_and(|schema| {
                schema.ends_with("routing-evaluation-results.schema.json")
                    && object.get("required").is_some()
                    && object.get("properties").is_some()
            });
    routing
        || (object.get("schema").and_then(Value::as_str)
            == Some("getcodexy.component-installation-cases.v1")
            && object.get("fixtures").is_some_and(Value::is_array))
}

fn routing_corpus(object: &serde_json::Map<String, Value>) -> bool {
    object.get("schema").and_then(Value::as_str) == Some("codexy.routing-evaluation-corpus.v1")
        && object
            .get("corpus_id")
            .and_then(Value::as_str)
            .is_some_and(|id| !id.is_empty())
        && object
            .get("tasks")
            .and_then(Value::as_array)
            .is_some_and(|tasks| {
                !tasks.is_empty()
                    && tasks.iter().all(|task| {
                        task.as_object().is_some_and(|task| {
                            ["id", "classification", "prompt", "acceptance_oracle"]
                                .iter()
                                .all(|key| {
                                    task.get(*key)
                                        .and_then(Value::as_str)
                                        .is_some_and(|value| !value.is_empty())
                                })
                        })
                    })
            })
}

fn routing_results(object: &serde_json::Map<String, Value>) -> bool {
    object.get("schema").and_then(Value::as_str) == Some("codexy.routing-evaluation-results.v1")
        && object
            .get("corpus_id")
            .and_then(Value::as_str)
            .is_some_and(|id| !id.is_empty())
        && object
            .get("selected_effort")
            .and_then(Value::as_str)
            .is_some()
        && object
            .get("results")
            .and_then(Value::as_array)
            .is_some_and(|results| !results.is_empty())
}

#[cfg(test)]
#[path = "density/tests.rs"]
mod tests;
