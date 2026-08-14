use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context as _, Result, bail};
use serde_json::Value;

mod analysis;
use analysis::{markdown_nonprose_lines, reason};
/// Returns a diagnostic only for a changed, maintained source line whose
/// syntax packs several executable statements or fields into one construct.
/// Line width is intentionally not an input: long identifiers, URLs, and
/// protocol literals are not evidence of LOC-compliance compression.
pub(super) fn error(path: &Path, text: &str) -> Option<String> {
    if source_disposition(path, text) != Disposition::Maintained {
        return None;
    }
    text.lines()
        .zip(
            rust_raw_string_lines(path, text).into_iter().zip(
                awk_program_lines(path, text)
                    .into_iter()
                    .zip(markdown_nonprose_lines(path, text)),
            ),
        )
        .enumerate()
        .find_map(|(index, (line, (raw_string, (awk_program, markdown_nonprose))))| {
            (!raw_string && !awk_program && !markdown_nonprose)
                .then(|| reason(path, line))
                .flatten()
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
        for (index, (line, raw_string)) in text
            .lines()
            .zip(
                rust_raw_string_lines(&path, &text).into_iter().zip(
                    awk_program_lines(&path, &text)
                        .into_iter()
                        .zip(markdown_nonprose_lines(&path, &text)),
                ),
            )
            .enumerate()
        {
            let (raw_string, (awk_program, markdown_nonprose)) = raw_string;
            let reason = (!raw_string && !awk_program && !markdown_nonprose)
                .then(|| reason(&path, line))
                .flatten();
            let structural = reason.is_some();
            if !structural && line.chars().count() <= 160 {
                continue;
            }
            let classification = match disposition {
                Disposition::Maintained if structural => "confirmed-density-defect",
                Disposition::Maintained => "maintained-readable",
                Disposition::ExactFixture => "exact-fixture",
                Disposition::ExactMalformedFixture => "exact-malformed-fixture",
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
                    "wide-line"
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
    ExactMalformedFixture,
    Generated,
    Vendor,
}

pub(super) fn disposition(path: &Path) -> Disposition {
    let path = path.to_string_lossy();
    let lower = path.to_ascii_lowercase();
    if lower.starts_with("vendor/") || lower.contains("/vendor/") || lower.contains("node_modules/")
    {
        Disposition::Vendor
    } else if lower.starts_with(".codex/") {
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
    let path = path.to_string_lossy();
    if is_fixture_path(&path) && text.starts_with("# codexy-exact-fixture: malformed\n") {
        return Disposition::ExactMalformedFixture;
    }
    if text.starts_with("// codexy-exact-fixture-file: ") {
        return Disposition::ExactFixture;
    }
    let exact_json_fixture = is_fixture_path(&path) && path.ends_with(".json");
    let routing_reference = path.contains("routing-evaluation-") && path.ends_with(".json");
    if (exact_json_fixture || routing_reference) && serde_json::from_str::<Value>(text).is_ok() {
        Disposition::ExactFixture
    } else {
        Disposition::Maintained
    }
}

fn is_fixture_path(path: &str) -> bool {
    path.starts_with("tests/fixtures/") || path.contains("/tests/fixtures/")
}

fn rust_raw_string_lines(path: &Path, text: &str) -> Vec<bool> {
    if path.extension().is_none_or(|extension| extension != "rs") {
        return vec![false; text.lines().count()];
    }
    let mut terminator = None;
    text.lines()
        .map(|line| {
            let was_raw = terminator.is_some();
            if let Some(end) = &terminator {
                if line.contains(end) {
                    terminator = None;
                }
            } else if let Some((hashes, suffix)) = (0..=8).find_map(|hashes| {
                let opener = format!("r{}\"", "#".repeat(hashes));
                line.split_once(&opener).map(|(_, suffix)| (hashes, suffix))
            }) {
                let end = format!("\"{}", "#".repeat(hashes));
                if !suffix.contains(&end) {
                    terminator = Some(end);
                    return true;
                }
            }
            was_raw
        })
        .collect()
}

fn awk_program_lines(path: &Path, text: &str) -> Vec<bool> {
    if path.extension().is_none_or(|extension| extension != "sh") {
        return vec![false; text.lines().count()];
    }
    let mut active = false;
    text.lines()
        .map(|line| {
            if !active && line.contains("awk ") {
                let quotes = line.matches('\'').count();
                active = quotes % 2 == 1;
                return active;
            }
            if active && line.trim() == "'" {
                active = false;
            }
            active
        })
        .collect()
}

#[cfg(test)]
#[path = "density/tests.rs"]
mod tests;
