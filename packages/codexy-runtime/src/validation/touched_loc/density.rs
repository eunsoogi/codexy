use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context as _, Result, bail};
use serde_json::Value;

const PROVENANCE: &str = include_str!("density/provenance_manifest.json");

/// Detects compact, executable structures in maintained changed source. This
/// intentionally recognizes only stable single-line forms; other syntax is
/// left to the complete source inventory and human readability review.
pub(super) fn error(path: &Path, text: &str) -> Option<String> {
    if disposition(path, text) != Disposition::Maintained {
        return None;
    }
    text.lines().enumerate().find_map(|(index, line)| {
        (admitted(path, line)).then(|| reason(path, line)).flatten().map(|reason| {
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

/// Reports every recognized structural candidate with its source-backed
/// disposition. It does not turn line length into a readability policy.
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
        let disposition = disposition(&path, &text);
        for (index, line) in text.lines().enumerate() {
            if reason(&path, line).is_none() {
                continue;
            }
            let classification = if disposition == Disposition::Maintained && !admitted(&path, line)
            {
                "maintained-readable/manual-audit"
            } else {
                disposition.name()
            };
            records.push(format!(
                "{}:{}\t{}\taudit-input=structural-density",
                path.display(),
                index + 1,
                classification,
            ));
        }
    }
    records.sort();
    Ok(records)
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum Disposition {
    Maintained,
    ExactFixture,
    Generated,
    Vendor,
}

impl Disposition {
    fn name(self) -> &'static str {
        match self {
            Self::Maintained => "confirmed-density-defect",
            Self::ExactFixture => "exact-fixture",
            Self::Generated => "generated",
            Self::Vendor => "vendor",
        }
    }
}

fn disposition(path: &Path, text: &str) -> Disposition {
    let path_text = path.to_string_lossy();
    if path_text.starts_with("vendor/") || path_text.contains("/vendor/") {
        return Disposition::Vendor;
    }
    if path_text.starts_with("target/") || path_text.contains("/target/") {
        return Disposition::Generated;
    }
    provenance(path, text).unwrap_or(Disposition::Maintained)
}

fn provenance(path: &Path, text: &str) -> Option<Disposition> {
    let document = serde_json::from_str::<Value>(PROVENANCE).ok()?;
    let source = document.get("sources")?.as_array()?.iter().find(|source| {
        source.get("path").and_then(Value::as_str) == path.to_str() && matches_source(source, text)
    })?;
    match source.get("classification").and_then(Value::as_str) {
        Some("exact-fixture") => Some(Disposition::ExactFixture),
        Some("generated") => Some(Disposition::Generated),
        _ => None,
    }
}

fn matches_source(source: &Value, text: &str) -> bool {
    let Ok(document) = serde_json::from_str::<Value>(text) else {
        return false;
    };
    source
        .get("schema")
        .and_then(Value::as_str)
        .is_some_and(|schema| document.get("schema").and_then(Value::as_str) == Some(schema))
        || source
            .get("marker")
            .and_then(Value::as_str)
            .is_some_and(|marker| {
                document.get("description").and_then(Value::as_str) == Some(marker)
            })
}

fn reason(path: &Path, line: &str) -> Option<&'static str> {
    let extension = path.extension().and_then(|extension| extension.to_str())?;
    let visible = visible_code(extension, line);
    match extension {
        "rs" if visible.contains('{') && separators(&visible, ';') >= 3 => {
            Some("dense Rust statements")
        }
        "py" | "js" | "ts" | "tsx" | "jsx" if separators(&visible, ';') >= 2 => {
            Some("dense executable statements")
        }
        "sh" | "ps1" if command_separators(&visible) >= 2 => Some("dense command chain"),
        "json" if fields(&visible, ':') >= 4 => Some("dense JSON object"),
        "toml" if fields(&visible, '=') >= 4 => Some("dense TOML table"),
        "yml" | "yaml" if fields(&visible, ':') >= 4 => Some("dense YAML flow mapping"),
        "md" if markdown_clauses(line) >= 3 => Some("dense Markdown clauses"),
        _ => None,
    }
}

fn admitted(path: &Path, line: &str) -> bool {
    let Some(extension) = path.extension().and_then(|extension| extension.to_str()) else {
        return false;
    };
    let trimmed = line.trim_start();
    match extension {
        "rs" => trimmed.starts_with("fn ") || trimmed.starts_with("pub fn "),
        "py" | "js" | "ts" | "tsx" | "jsx" => !line.contains(['\'', '"']),
        "sh" | "ps1" => simple_shell_chain(trimmed),
        "json" => {
            !path.to_string_lossy().contains("/references/")
                && !path.to_string_lossy().contains("/templates/")
                && !path.to_string_lossy().starts_with(".agents/")
        }
        "toml" => true,
        "md" => !line.starts_with(' ') && !trimmed.starts_with('|'),
        _ => false,
    }
}

fn simple_shell_chain(line: &str) -> bool {
    let normalized = line.replace("&&", ";").replace("||", ";");
    let parts = normalized.split(';').collect::<Vec<_>>();
    parts.len() >= 3
        && parts
            .iter()
            .all(|part| !part.trim().is_empty() && !part.trim().contains(char::is_whitespace))
}

fn visible_code(extension: &str, line: &str) -> String {
    let slash_comments = matches!(extension, "rs" | "js" | "ts" | "tsx" | "jsx");
    let hash_comments = matches!(extension, "py" | "sh" | "ps1" | "yml" | "yaml");
    let mut visible = String::new();
    let mut characters = line.chars().peekable();
    let mut quote = None;
    while let Some(character) = characters.next() {
        if let Some(delimiter) = quote {
            if character == '\\' {
                characters.next();
            } else if character == delimiter {
                quote = None;
            }
        } else if matches!(character, '\'' | '"') {
            quote = Some(character);
        } else if (hash_comments && character == '#')
            || (slash_comments && character == '/' && characters.peek() == Some(&'/'))
        {
            break;
        } else {
            visible.push(character);
        }
    }
    visible
}

fn separators(line: &str, separator: char) -> usize {
    line.matches(separator).count()
}

fn command_separators(line: &str) -> usize {
    line.replace(";;", "").matches(';').count()
        + line.matches("&&").count()
        + line.matches("||").count()
}

fn fields(line: &str, separator: char) -> usize {
    line.contains('{')
        .then(|| separators(line, separator))
        .unwrap_or_default()
}

fn markdown_clauses(line: &str) -> usize {
    line.split(';')
        .filter(|clause| clause.split_whitespace().count() >= 3)
        .count()
}
