use std::fs;
use std::path::{Path, PathBuf};

use super::files::code_extensions;

pub(super) fn candidates(candidate: &Path, from_extension: Option<&str>) -> Vec<PathBuf> {
    let extension = candidate.extension().and_then(|item| item.to_str());
    if extension.is_some() {
        let mut output = vec![candidate.to_path_buf()];
        if matches!(extension, Some("js" | "jsx" | "mjs" | "cjs")) {
            let stem = candidate.with_extension("");
            output.push(stem.with_extension("ts"));
            output.push(stem.with_extension("tsx"));
        }
        return output;
    }
    let mut output = go_directory_candidates(candidate, from_extension);
    output.push(candidate.to_path_buf());
    let extensions = code_extensions();
    let from = from_extension.map(|item| format!(".{item}"));
    if let Some(from) = from {
        if extensions.contains(&from) {
            output.push(candidate.with_extension(from.trim_start_matches('.')));
        }
    }
    output.extend(
        extensions
            .iter()
            .map(|extension| candidate.with_extension(extension.trim_start_matches('.'))),
    );
    output.extend(
        extensions
            .iter()
            .map(|extension| candidate.join(format!("index{extension}"))),
    );
    output.push(candidate.join("__init__.py"));
    output.push(candidate.join("mod.rs"));
    output
}

fn go_directory_candidates(candidate: &Path, from_extension: Option<&str>) -> Vec<PathBuf> {
    if from_extension != Some("go") || !candidate.is_dir() {
        return Vec::new();
    }
    let mut go_files = fs::read_dir(candidate)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|item| item.to_str()) == Some("go"))
        .collect::<Vec<_>>();
    go_files.sort();
    go_files
}
