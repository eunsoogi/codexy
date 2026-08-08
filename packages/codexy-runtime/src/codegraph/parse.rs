use std::collections::BTreeSet;
use std::path::Path;

use regex::Regex;

use super::GraphFile;
use super::files::read_source;
use super::language::parse_language;
use super::mask::code_position_mask;

pub(super) fn parse_file(root: &Path, file: &str, indexed_files: &BTreeSet<String>) -> GraphFile {
    let extension = Path::new(file)
        .extension()
        .and_then(|value| value.to_str())
        .map_or(String::new(), |value| format!(".{value}"));
    let parsed = if matches!(
        extension.as_str(),
        ".js" | ".jsx" | ".mjs" | ".cjs" | ".ts" | ".tsx"
    ) {
        parse_javascript(root, file)
    } else {
        parse_language(root, file, &extension, indexed_files)
    };
    GraphFile {
        path: file.to_owned(),
        imports: unique(parsed.0),
        exports: unique(parsed.1),
    }
}

fn parse_javascript(root: &Path, file: &str) -> (Vec<String>, Vec<String>) {
    let source = read_source(root, file);
    let mask = code_position_mask(&source);
    let imports = regex_values(
        &source,
        &mask,
        &[
            r#"\bimport\s*(?:[^"'()]*?\s*from\s*)?["']([^"']+)["']"#,
            r#"\bimport\s*\(\s*["']([^"']+)["'](?:\s*,[^)]*)?\s*\)"#,
            r#"\brequire\(\s*["']([^"']+)["']\s*\)"#,
            r#"\bexport\s*(?:type\s+)?\*\s*(?:as\s+[A-Za-z_$][\w$]*\s*)?from\s*["']([^"']+)["']"#,
            r#"\bexport\s*(?:type\s+)?\{[^}]+\}\s*from\s*["']([^"']+)["']"#,
        ],
    );
    let mut exports = regex_values(
        &source,
        &mask,
        &[
            r"\bexport\s+(?:(?:async\s+)?(?:function|class|const|let|var)|interface|type|enum)\s+([A-Za-z_$][\w$]*)",
        ],
    );
    exports.extend(export_list_values(&source, &mask));
    (imports, exports)
}

pub(super) fn parse_simple(
    source: &str,
    mask: &[bool],
    imports: &[&str],
    exports: &[&str],
) -> (Vec<String>, Vec<String>) {
    (
        regex_values(source, mask, imports),
        regex_values(source, mask, exports),
    )
}

pub(super) fn regex_values(source: &str, mask: &[bool], patterns: &[&str]) -> Vec<String> {
    let mut values = Vec::new();
    for pattern in patterns {
        let Some(regex) = Regex::new(pattern).ok() else {
            continue;
        };
        for caps in regex.captures_iter(source) {
            let Some(full) = caps.get(0) else { continue };
            if !mask.get(full.start()).copied().unwrap_or(false) {
                continue;
            }
            let capture = if caps.len() > 2 {
                caps.get(2).or_else(|| caps.get(1))
            } else {
                caps.get(1)
            };
            if let Some(value) = capture {
                values.push(value.as_str().to_owned());
            }
        }
    }
    values
}

fn export_list_values(source: &str, mask: &[bool]) -> Vec<String> {
    let Some(regex) = Regex::new(r"\bexport\s*(?:type\s+)?\{([^}]+)\}").ok() else {
        return Vec::new();
    };
    regex
        .captures_iter(source)
        .filter_map(|caps| {
            let full = caps.get(0)?;
            mask.get(full.start()).copied().filter(|value| *value)?;
            caps.get(1).map(|value| value.as_str().to_owned())
        })
        .flat_map(|list| export_list(&list))
        .collect()
}

pub(super) fn import_list(value: &str) -> Vec<String> {
    value
        .trim()
        .trim_start_matches('(')
        .trim_end_matches(')')
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(|item| item.split_whitespace().next().unwrap_or("").to_owned())
        .filter(|item| !item.is_empty())
        .collect()
}

fn export_list(value: &str) -> Vec<String> {
    value
        .trim()
        .trim_start_matches('(')
        .trim_end_matches(')')
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(|item| {
            item.split_once(" as ")
                .map_or(item, |(_, alias)| alias)
                .split_whitespace()
                .next()
                .unwrap_or("")
                .to_owned()
        })
        .filter(|item| !item.is_empty())
        .collect()
}

fn unique(mut values: Vec<String>) -> Vec<String> {
    values.sort();
    values.dedup();
    values
}
