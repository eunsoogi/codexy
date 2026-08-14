use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context as _, Result, bail};
/// Returns a diagnostic only for a changed, maintained source line whose
/// syntax packs several executable statements or fields into one construct.
/// Line width is intentionally not an input: long identifiers, URLs, and
/// protocol literals are not evidence of LOC-compliance compression.
pub(super) fn error(path: &Path, text: &str) -> Option<String> {
    text.lines().enumerate().find_map(|(index, line)| {
        if disposition_for(path, line) != Disposition::Maintained {
            return None;
        }
        reason(path, line).map(|reason| {
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

/// Classifies every wide-line audit candidate without using width as a failing
/// policy. The report is deliberately stable and source-addressable so a review
/// can distinguish an exact fixture or generated artifact from maintained code.
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
        for (index, line) in text.lines().enumerate() {
            if line.chars().count() <= 160 {
                continue;
            }
            let classification = match inventory_disposition(&path, line) {
                Disposition::Maintained if reason(&path, line).is_some() => {
                    "confirmed-density-defect"
                }
                Disposition::Maintained => "maintained-readable",
                Disposition::ExactFixture => "exact-fixture",
                Disposition::ExactMalformedFixture => "exact-malformed-fixture",
                Disposition::Generated => "generated",
                Disposition::Vendor => "vendor",
            };
            records.push(format!(
                "{}:{}\t{classification}\taudit-input=wide-line",
                path.display(),
                index + 1,
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
    } else if (lower.starts_with("tests/fixtures/") || lower.contains("/tests/fixtures/"))
        && [
            "invalid",
            "malformed",
            "broken",
            "corrupt",
            "unsafe",
            "missing",
        ]
        .iter()
        .any(|marker| lower.contains(marker))
    {
        Disposition::ExactMalformedFixture
    } else {
        Disposition::Maintained
    }
}

fn disposition_for(path: &Path, line: &str) -> Disposition {
    let base = disposition(path);
    if base != Disposition::Maintained {
        return base;
    }
    let path = path.to_string_lossy();
    let exact_json_fixture = path.contains("/tests/fixtures/") && path.ends_with(".json");
    let exact_test_literal = path.contains("/tests/") && line.contains('"');
    let routing_reference = path.contains("routing-evaluation-") && path.ends_with(".json");
    if exact_json_fixture || exact_test_literal || routing_reference {
        Disposition::ExactFixture
    } else {
        Disposition::Maintained
    }
}

fn inventory_disposition(path: &Path, line: &str) -> Disposition {
    let path_text = path.to_string_lossy();
    if path_text.contains("/tests/") || path_text.starts_with("tests/") {
        Disposition::ExactFixture
    } else {
        disposition_for(path, line)
    }
}

fn reason(path: &Path, line: &str) -> Option<&'static str> {
    let visible = visible_code(line);
    match path.extension().and_then(|extension| extension.to_str()) {
        Some("rs") if visible.contains('{') && statement_count(&visible) >= 3 => {
            Some("dense Rust statements")
        }
        Some("py" | "js" | "ts" | "tsx" | "jsx") if statement_count(&visible) >= 3 => {
            Some("dense executable statements")
        }
        Some("sh" | "ps1") if command_chain_count(&visible) >= 3 => Some("dense command chain"),
        Some("json") if inline_object_fields(&visible, ':') >= 4 => Some("dense JSON object"),
        Some("toml") if inline_object_fields(&visible, '=') >= 4 => Some("dense TOML table"),
        Some("yml" | "yaml") if yaml_flow_fields(&visible) >= 4 => Some("dense YAML flow mapping"),
        Some("yml" | "yaml") if command_chain_count(&visible) >= 3 => {
            Some("dense workflow command chain")
        }
        _ => None,
    }
}

fn visible_code(line: &str) -> String {
    let before_comment = line.split_once("//").map_or(line, |(code, _)| code);
    let mut visible = String::with_capacity(before_comment.len());
    let mut quoted = None;
    for character in before_comment.chars() {
        match (quoted, character) {
            (None, '"' | '\'') => quoted = Some(character),
            (Some(quote), current) if quote == current => quoted = None,
            (None, current) => visible.push(current),
            _ => {}
        }
    }
    visible
}

fn statement_count(line: &str) -> usize {
    line.matches(';').count() + 1
}

fn command_chain_count(line: &str) -> usize {
    line.replace(";;", "")
        .replace("; then", "")
        .replace("; fi", "")
        .matches(';')
        .count()
        + line.matches("&&").count()
        + line.matches("||").count()
        + 1
}

fn inline_object_fields(line: &str, separator: char) -> usize {
    let Some((_, inner)) = line.split_once('{') else {
        return 0;
    };
    let Some((inner, _)) = inner.split_once('}') else {
        return 0;
    };
    inner.matches(separator).count()
}

fn yaml_flow_fields(line: &str) -> usize {
    let Some((prefix, _)) = line.split_once('{') else {
        return 0;
    };
    prefix
        .trim_end()
        .ends_with(':')
        .then(|| inline_object_fields(line, ':'))
        .unwrap_or_default()
}
#[cfg(test)]
#[path = "density/tests.rs"]
mod tests;
