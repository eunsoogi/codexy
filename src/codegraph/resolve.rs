use std::collections::BTreeSet;
use std::path::Path;

use super::candidates::candidates;
use super::files::to_posix;
use super::path_ops::{normalize_path, normalize_posix, path_join_posix, pathdiff};

pub(super) struct ResolvedImport {
    pub(super) to: String,
    pub(super) resolved: bool,
}

pub(super) fn graph_path(root: &Path, input: &str) -> String {
    let path = Path::new(input);
    let relative = if path.is_absolute() {
        let absolute = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        if let Ok(relative) = absolute.strip_prefix(root) {
            relative.to_path_buf()
        } else {
            return format!("/{}", normalize_posix(&to_posix(&absolute)));
        }
    } else {
        path.to_path_buf()
    };
    normalize_posix(&to_posix(&relative))
}

pub(super) fn normalize_language_import(
    extension: &str,
    specifier: &str,
    file: &str,
    package_name: Option<&str>,
) -> String {
    match extension {
        ".py" => normalize_python(specifier, file),
        ".rs" => normalize_rust(specifier, file),
        ".go" => normalize_go_import(specifier, file, package_name),
        ".java" | ".kt" => normalize_package(specifier, file, package_name),
        _ if specifier.starts_with('.') => specifier.to_owned(),
        _ => format!("./{specifier}"),
    }
}

pub(super) fn normalize_go_import(
    specifier: &str,
    file: &str,
    package_name: Option<&str>,
) -> String {
    if specifier.starts_with('.') {
        return specifier.to_owned();
    }
    let Some(package_name) = package_name else {
        return specifier.to_owned();
    };
    if specifier != package_name && !specifier.starts_with(&format!("{package_name}/")) {
        return specifier.to_owned();
    }
    let suffix = specifier
        .trim_start_matches(package_name)
        .trim_start_matches('/');
    relative_from(file, if suffix.is_empty() { "." } else { suffix })
}

pub(super) fn resolve_import(
    root: &Path,
    from_file: &str,
    specifier: &str,
    indexed_files: &BTreeSet<String>,
) -> ResolvedImport {
    if !specifier.starts_with('.') {
        return ResolvedImport {
            to: specifier.to_owned(),
            resolved: false,
        };
    }
    let from_dir = path_join_posix(root, from_file)
        .parent()
        .map_or_else(|| root.to_path_buf(), Path::to_path_buf);
    let candidate = normalize_path(&from_dir.join(specifier));
    for absolute in candidates(
        &candidate,
        Path::new(from_file)
            .extension()
            .and_then(|item| item.to_str()),
    ) {
        if absolute.is_file() {
            let relative = absolute
                .strip_prefix(root)
                .map_or_else(|_| to_posix(&absolute), to_posix);
            if indexed_files.contains(&relative) {
                return ResolvedImport {
                    to: relative,
                    resolved: true,
                };
            }
        }
    }
    ResolvedImport {
        to: specifier.to_owned(),
        resolved: false,
    }
}

fn normalize_python(specifier: &str, file: &str) -> String {
    if specifier.starts_with('.') {
        let dots = specifier.chars().take_while(|ch| *ch == '.').count();
        let rest = specifier
            .trim_start_matches('.')
            .replace('.', "/")
            .trim_matches('/')
            .to_owned();
        let parents = "../".repeat(dots.saturating_sub(1));
        if rest.is_empty() {
            format!("./{parents}")
        } else {
            format!("./{parents}{rest}")
        }
    } else {
        relative_from(file, &specifier.replace('.', "/"))
    }
}

fn normalize_rust(specifier: &str, file: &str) -> String {
    if let Some(rest) = specifier.strip_prefix("crate::") {
        return relative_from(
            file,
            &format!("{}/{}", rust_crate_root(file), rest.replace("::", "/")),
        );
    }
    if let Some(rest) = specifier.strip_prefix("super::") {
        return format!("./../{}", rest.replace("::", "/"));
    }
    if let Some(rest) = specifier.strip_prefix("self::") {
        return format!("./{}", rest.replace("::", "/"));
    }
    let file_path = Path::new(file);
    let dirname = file_path.parent().map(to_posix).unwrap_or_default();
    let basename = file_path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    if (dirname == "src" || dirname.starts_with("src/"))
        && !matches!(basename, "lib.rs" | "main.rs" | "mod.rs")
    {
        let stem = file_path
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or_default();
        format!("./{stem}/{specifier}")
    } else {
        format!("./{}", specifier.replace("::", "/"))
    }
}

fn normalize_package(specifier: &str, file: &str, package_name: Option<&str>) -> String {
    let import_path = specifier.replace('.', "/");
    let Some(package_path) = package_name.map(|name| name.replace('.', "/")) else {
        return format!("./{import_path}");
    };
    let file_dir = Path::new(file).parent().map(to_posix).unwrap_or_default();
    let source_root = file_dir
        .strip_suffix(&package_path)
        .map_or("", |prefix| prefix.trim_end_matches('/'));
    let target = if source_root.is_empty() {
        import_path
    } else {
        format!("{source_root}/{import_path}")
    };
    relative_from(file, &target)
}

fn relative_from(file: &str, target: &str) -> String {
    let from_dir = Path::new(file).parent().unwrap_or_else(|| Path::new(""));
    let relative = pathdiff(from_dir, Path::new(target));
    if relative.starts_with('.') {
        relative
    } else {
        format!("./{relative}")
    }
}

fn rust_crate_root(file: &str) -> String {
    let parts = file.split('/').collect::<Vec<_>>();
    parts
        .iter()
        .position(|part| *part == "src")
        .map_or_else(|| ".".to_owned(), |index| parts[..=index].join("/"))
}
