use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use regex::Regex;

use super::files::read_source;
use super::markup::{parse_markup, parse_stylesheet};
use super::mask::language_mask;
use super::parse::{import_list, parse_simple, regex_values};
use super::python::parse_python;
use super::resolve::{normalize_go_import, normalize_language_import};

pub(super) fn parse_language(
    root: &Path,
    file: &str,
    extension: &str,
    indexed_files: &BTreeSet<String>,
) -> (Vec<String>, Vec<String>) {
    let source = read_source(root, file);
    match extension {
        ".html" | ".htm" | ".svg" => parse_markup(&source),
        ".css" | ".scss" | ".sass" | ".less" => parse_stylesheet(&source),
        _ => {
            let mask = language_mask(&source, extension);
            parse_masked_language(root, file, extension, indexed_files, &source, &mask)
        }
    }
}

fn parse_masked_language(
    root: &Path,
    file: &str,
    extension: &str,
    indexed_files: &BTreeSet<String>,
    source: &str,
    mask: &[bool],
) -> (Vec<String>, Vec<String>) {
    match extension {
        ".py" => parse_python(root, file, source, mask, indexed_files),
        ".go" => parse_go(root, file, source, mask),
        ".rs" => parse_rust(file, source, mask),
        ".rb" => parse_simple(
            source,
            mask,
            &[
                r#"\brequire_relative\s+["']([^"']+)["']"#,
                r#"\brequire\s+["'](\.[^"']+)["']"#,
            ],
            &[r"\b(?:class|module|def)\s+([A-Z]\w*|[a-z_]\w*[!?=]?)"],
        ),
        ".java" | ".kt" => parse_package_language(file, extension, source, mask),
        _ => (Vec::new(), Vec::new()),
    }
}

fn parse_go(root: &Path, file: &str, source: &str, mask: &[bool]) -> (Vec<String>, Vec<String>) {
    let module_path = read_go_module_path(root);
    let mut imports = regex_values(
        source,
        mask,
        &[r#"\bimport\s+(?:[A-Za-z_]\w*\s+)?["']([^"']+)["']"#],
    )
    .into_iter()
    .map(|item| normalize_go_import(&item, file, module_path.as_deref()))
    .collect::<Vec<_>>();
    imports.extend(go_block_imports(source, mask, file, module_path.as_deref()));
    let exports = regex_values(source, mask, &[r"\b(?:func|type|var|const)\s+([A-Z]\w*)"]);
    (imports, exports)
}

fn go_block_imports(
    source: &str,
    mask: &[bool],
    file: &str,
    module_path: Option<&str>,
) -> Vec<String> {
    let Some(block_regex) = Regex::new(r"\bimport\s*\(([\s\S]*?)\)").ok() else {
        return Vec::new();
    };
    block_regex
        .captures_iter(source)
        .filter_map(|caps| {
            let block = caps.get(1)?;
            mask.get(block.start()).copied().filter(|value| *value)?;
            let block_mask = mask.get(block.start()..block.end())?;
            Some(regex_values(
                block.as_str(),
                block_mask,
                &[r#"(?m)^\s*(?:(?:[A-Za-z_]\w*|\.)\s+)?["']([^"']+)["']"#],
            ))
        })
        .flatten()
        .map(|item| normalize_go_import(&item, file, module_path))
        .collect()
}

fn parse_rust(file: &str, source: &str, mask: &[bool]) -> (Vec<String>, Vec<String>) {
    let mut imports = regex_values(source, mask, &[r"\bmod\s+([A-Za-z_]\w*)\s*;"])
        .into_iter()
        .map(|item| normalize_language_import(".rs", &item, file, None))
        .collect::<Vec<_>>();
    let Some(use_regex) = Regex::new(
        r"\buse\s+((?:crate|self|super)::[A-Za-z_]\w*(?:::[A-Za-z_]\w*)*)(?:::\{([^}]+)\})?",
    )
    .ok() else {
        return (imports, Vec::new());
    };
    for caps in use_regex.captures_iter(source) {
        let Some(full) = caps.get(0) else { continue };
        if !mask.get(full.start()).copied().unwrap_or(false) {
            continue;
        }
        let base = caps.get(1).map_or("", |item| item.as_str());
        if let Some(group) = caps.get(2) {
            imports.extend(import_list(group.as_str()).into_iter().map(|target| {
                normalize_language_import(".rs", &format!("{base}::{target}"), file, None)
            }));
        } else {
            imports.push(normalize_language_import(".rs", base, file, None));
        }
    }
    let exports = regex_values(
        source,
        mask,
        &[r"\bpub\s+(?:fn|struct|enum|trait|mod|const|static)\s+([A-Za-z_]\w*)"],
    );
    (imports, exports)
}

fn parse_package_language(
    file: &str,
    extension: &str,
    source: &str,
    mask: &[bool],
) -> (Vec<String>, Vec<String>) {
    let package_name = Regex::new(r"(?m)^\s*package\s+([A-Za-z_]\w*(?:\.[A-Za-z_]\w*)*)")
        .ok()
        .and_then(|regex| regex.captures(source))
        .and_then(|caps| caps.get(1).map(|item| item.as_str().to_owned()));
    let imports = if extension == ".java" {
        java_imports(source, mask)
    } else {
        regex_values(
            source,
            mask,
            &[r"\bimport\s+([A-Za-z_]\w*(?:\.[A-Za-z_]\w*)+)"],
        )
    }
    .into_iter()
    .map(|item| normalize_language_import(extension, &item, file, package_name.as_deref()))
    .collect::<Vec<_>>();
    let exports = regex_values(
        source,
        mask,
        if extension == ".java" {
            &[r"\b(?:class|interface|enum|record)\s+([A-Za-z_]\w*)"]
        } else {
            &[r"\b(?:class|interface|object|fun|val|var)\s+([A-Za-z_]\w*)"]
        },
    );
    (imports, exports)
}

fn java_imports(source: &str, mask: &[bool]) -> Vec<String> {
    let Some(regex) =
        Regex::new(r"\bimport\s+(static\s+)?([A-Za-z_]\w*(?:\.[A-Za-z_]\w*)+)\s*;").ok()
    else {
        return Vec::new();
    };
    regex
        .captures_iter(source)
        .filter_map(|caps| {
            let full = caps.get(0)?;
            mask.get(full.start()).copied().filter(|value| *value)?;
            let import_path = caps.get(2)?.as_str();
            if caps.get(1).is_some() {
                return import_path
                    .rsplit_once('.')
                    .map(|(class_path, _)| class_path.to_owned());
            }
            Some(import_path.to_owned())
        })
        .collect()
}

fn read_go_module_path(root: &Path) -> Option<String> {
    fs::read_to_string(root.join("go.mod"))
        .ok()
        .and_then(|text| {
            Regex::new(r"(?m)^\s*module\s+(\S+)")
                .ok()?
                .captures(&text)?
                .get(1)
                .map(|item| item.as_str().to_owned())
        })
}
