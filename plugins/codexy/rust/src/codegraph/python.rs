use std::collections::BTreeSet;
use std::path::Path;

use regex::Regex;

use super::parse::{import_list, regex_values};
use super::resolve::{normalize_language_import, resolve_import};

pub(super) fn parse_python(
    root: &Path,
    file: &str,
    source: &str,
    mask: &[bool],
    indexed_files: &BTreeSet<String>,
) -> (Vec<String>, Vec<String>) {
    let mut imports = Vec::new();
    for (pattern, has_import_list) in PYTHON_IMPORT_PATTERNS {
        let Some(regex) = Regex::new(pattern).ok() else {
            continue;
        };
        for caps in regex.captures_iter(source) {
            let Some(full) = caps.get(0) else { continue };
            if !mask.get(full.start()).copied().unwrap_or(false) {
                continue;
            }
            if *has_import_list {
                let base = caps.get(1).map_or("", |item| item.as_str());
                collect_from_imports(root, file, indexed_files, base, caps.get(2), &mut imports);
            } else {
                imports.extend(
                    import_list(caps.get(1).map_or("", |item| item.as_str()))
                        .iter()
                        .map(|target| normalize_language_import(".py", target, file, None)),
                );
            }
        }
    }
    let exports = regex_values(source, mask, &[r"\b(?:def|class)\s+([A-Za-z_]\w*)"]);
    (
        imports.into_iter().filter(|item| item != "./").collect(),
        exports,
    )
}

fn collect_from_imports(
    root: &Path,
    file: &str,
    indexed_files: &BTreeSet<String>,
    base: &str,
    targets: Option<regex::Match<'_>>,
    imports: &mut Vec<String>,
) {
    for target in import_list(targets.map_or("", |item| item.as_str())) {
        let candidate = if base.chars().all(|ch| ch == '.') {
            format!("{base}{target}")
        } else {
            format!("{base}.{target}")
        };
        let submodule = normalize_language_import(".py", &candidate, file, None);
        if resolve_import(root, file, &submodule, indexed_files).resolved {
            imports.push(submodule);
        } else {
            imports.push(normalize_language_import(".py", base, file, None));
        }
    }
}

const PYTHON_IMPORT_PATTERNS: &[(&str, bool)] = &[
    (
        r"\bfrom\s+(\.+)\s+import\s+(\([\s\S]*?\)|[A-Za-z_]\w*(?:\s+as\s+[A-Za-z_]\w*)?(?:\s*,\s*[A-Za-z_]\w*(?:\s+as\s+[A-Za-z_]\w*)?)*)",
        true,
    ),
    (
        r"\bfrom\s+((?:\.+)?[A-Za-z_]\w*(?:\.[A-Za-z_]\w*)*)\s+import\s+(\([\s\S]*?\)|[A-Za-z_]\w*(?:\s+as\s+[A-Za-z_]\w*)?(?:\s*,\s*[A-Za-z_]\w*(?:\s+as\s+[A-Za-z_]\w*)?)*)",
        true,
    ),
    (
        r"(?m)^\s*import\s+([A-Za-z_]\w*(?:\.[A-Za-z_]\w*)*(?:\s+as\s+[A-Za-z_]\w*)?(?:\s*,\s*[A-Za-z_]\w*(?:\.[A-Za-z_]\w*)*(?:\s+as\s+[A-Za-z_]\w*)?)*)",
        false,
    ),
];
