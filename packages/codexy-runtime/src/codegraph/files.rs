use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use ignore::WalkBuilder;

const CODE_EXTENSIONS: &[&str] = &[
    "js", "jsx", "ts", "tsx", "mjs", "cjs", "py", "go", "rs", "rb", "java", "kt", "html", "htm",
    "css", "scss", "sass", "less", "svg", "vue", "svelte", "astro", "json", "jsonc", "yaml", "yml",
    "toml", "md", "mdx",
];

pub(super) fn result_limit(input: Option<usize>) -> usize {
    input.filter(|value| *value > 0).unwrap_or(80)
}

pub(super) fn repo_root(input_root: Option<&str>) -> PathBuf {
    let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let candidate = input_root.map_or_else(|| current_dir.clone(), PathBuf::from);
    let rooted = if candidate.is_absolute() {
        candidate
    } else {
        current_dir.join(candidate)
    };
    if rooted.exists() {
        rooted.canonicalize().unwrap_or(rooted)
    } else {
        current_dir
    }
}

pub(super) fn is_code_file(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| CODE_EXTENSIONS.contains(&extension))
}

pub(super) fn code_extensions() -> BTreeSet<String> {
    CODE_EXTENSIONS
        .iter()
        .map(|extension| format!(".{extension}"))
        .collect()
}

pub(super) fn to_posix(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

pub(super) fn walk_code_files(root: &Path) -> Vec<String> {
    let mut files = WalkBuilder::new(root)
        .hidden(false)
        .filter_entry(|entry| {
            let name = entry.file_name().to_string_lossy();
            name != ".git" && name != "node_modules"
        })
        .build()
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .file_type()
                .is_some_and(|file_type| file_type.is_file())
        })
        .filter(|entry| is_code_file(entry.path()))
        .filter_map(|entry| entry.path().strip_prefix(root).ok().map(to_posix))
        .collect::<Vec<_>>();
    files.sort();
    files
}

pub(super) fn read_source(root: &Path, file: &str) -> String {
    fs::read_to_string(path_join_posix(root, file)).unwrap_or_default()
}

fn path_join_posix(root: &Path, file: &str) -> PathBuf {
    file.split('/')
        .fold(root.to_path_buf(), |path, part| path.join(part))
}
