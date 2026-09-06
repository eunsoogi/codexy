use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use ignore::WalkBuilder;
use sha2::{Digest, Sha256};

use super::errors::{CodegraphError, CodegraphErrorKind, record, remember_files, was_discovered};
use super::snapshot::{FileSnapshot, environment_digest};

const CODE_EXTENSIONS: &[&str] = &[
    "js", "jsx", "ts", "tsx", "mjs", "cjs", "py", "go", "rs", "rb", "java", "kt", "html", "htm",
    "css", "scss", "sass", "less", "svg", "vue", "svelte", "astro", "json", "jsonc", "yaml", "yml",
    "toml", "md", "mdx",
];

#[derive(Debug)]
pub(super) struct SourceSnapshot {
    pub(super) source: String,
    pub(super) digest: [u8; 32],
}

pub(super) fn result_limit(input: Option<usize>) -> usize {
    input.filter(|value| *value > 0).unwrap_or(80)
}

pub(super) fn repo_root(input_root: Option<&str>) -> Result<PathBuf, CodegraphError> {
    let current_dir = std::env::current_dir().map_err(|error| {
        CodegraphError::new(
            CodegraphErrorKind::RootUnreadable,
            ".",
            format!("unable to determine current directory: {error}"),
        )
    })?;
    let Some(input_root) = input_root else {
        return Ok(current_dir);
    };
    if input_root.is_empty() {
        return Err(CodegraphError::new(
            CodegraphErrorKind::RootMissing,
            input_root,
            "repository root must not be empty",
        ));
    }
    let candidate = PathBuf::from(input_root);
    let rooted = if candidate.is_absolute() {
        candidate
    } else {
        current_dir.join(candidate)
    };
    let metadata = fs::metadata(&rooted).map_err(|error| root_error(&rooted, error))?;
    if !metadata.is_dir() {
        return Err(CodegraphError::new(
            CodegraphErrorKind::RootNotDirectory,
            to_posix(&rooted),
            "repository root is not a directory",
        ));
    }
    fs::read_dir(&rooted).map_err(|error| root_error(&rooted, error))?;
    rooted
        .canonicalize()
        .map_err(|error| root_error(&rooted, error))
}

fn root_error(path: &Path, error: io::Error) -> CodegraphError {
    let kind = if error.kind() == io::ErrorKind::NotFound {
        CodegraphErrorKind::RootMissing
    } else {
        CodegraphErrorKind::RootUnreadable
    };
    CodegraphError::new(kind, to_posix(path), error.to_string())
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
    path.to_string_lossy()
        .replace(std::path::MAIN_SEPARATOR, "/")
}

pub(super) fn walk_code_files(root: &Path) -> Vec<String> {
    enumerate_code_files(root)
}

pub(super) fn discover_code_files(root: &Path) -> FileSnapshot {
    FileSnapshot {
        files: enumerate_code_files(root),
        environment_digest: environment_digest(root),
    }
}

fn enumerate_code_files(root: &Path) -> Vec<String> {
    let mut files = WalkBuilder::new(root)
        .hidden(false)
        .filter_entry(|entry| {
            let name = entry.file_name().to_string_lossy();
            name != ".git" && name != "node_modules"
        })
        .build()
        .filter_map(|result| match result {
            Ok(entry) => Some(entry),
            Err(error) => {
                record_walk_error(root, &error);
                None
            }
        })
        .filter(|entry| {
            entry
                .file_type()
                .is_some_and(|file_type| file_type.is_file())
        })
        .filter(|entry| is_code_file(entry.path()))
        .filter_map(|entry| entry.path().strip_prefix(root).ok().map(to_posix))
        .collect::<Vec<_>>();
    files.sort();
    remember_files(&files);
    files
}

pub(super) fn read_source_snapshot(root: &Path, file: &str) -> Option<SourceSnapshot> {
    let source = read_source_text(root, file)?;
    Some(SourceSnapshot {
        digest: Sha256::digest(source.as_bytes()).into(),
        source,
    })
}

pub(super) fn read_source(root: &Path, file: &str) -> String {
    read_source_text(root, file).unwrap_or_default()
}

fn read_source_text(root: &Path, file: &str) -> Option<String> {
    let path = path_join_posix(root, file);
    let display_path = relative_display(root, &path);
    match fs::read_to_string(&path) {
        Ok(source) => Some(source),
        Err(error) => {
            record(CodegraphError::new(
                file_error_kind(&error, was_discovered(&display_path)),
                display_path,
                error.to_string(),
            ));
            None
        }
    }
}

fn file_error_kind(error: &io::Error, was_discovered: bool) -> CodegraphErrorKind {
    match error.kind() {
        io::ErrorKind::NotFound if was_discovered => CodegraphErrorKind::FileDeletionRace,
        io::ErrorKind::NotFound => CodegraphErrorKind::SourceMissing,
        io::ErrorKind::PermissionDenied => CodegraphErrorKind::PermissionDenied,
        io::ErrorKind::InvalidData => CodegraphErrorKind::EncodingFailure,
        _ => CodegraphErrorKind::ReadFailure,
    }
}

fn record_walk_error(root: &Path, error: &ignore::Error) {
    let kind = error
        .io_error()
        .map_or(CodegraphErrorKind::WalkFailure, |error| {
            match error.kind() {
                io::ErrorKind::PermissionDenied => CodegraphErrorKind::PermissionDenied,
                io::ErrorKind::InvalidData => CodegraphErrorKind::EncodingFailure,
                _ => CodegraphErrorKind::WalkFailure,
            }
        });
    record(CodegraphError::new(kind, to_posix(root), error.to_string()));
}

fn relative_display(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .map_or_else(|_| to_posix(path), to_posix)
}

fn path_join_posix(root: &Path, file: &str) -> PathBuf {
    let candidate = Path::new(file);
    if candidate.is_absolute() {
        return candidate.to_path_buf();
    }
    file.split('/')
        .fold(root.to_path_buf(), |path, part| path.join(part))
}

#[cfg(test)]
mod tests {
    use super::super::errors::{begin_operation, take_errors};
    use super::*;

    #[test]
    fn read_source_distinguishes_deletion_races_from_missing_sources()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = tempfile::tempdir()?;
        let source = root.path().join("deleted.rs");
        std::fs::write(&source, "pub const DELETED: u8 = 1;\n")?;
        begin_operation();
        assert_eq!(walk_code_files(root.path()), vec!["deleted.rs"]);
        std::fs::remove_file(source)?;
        assert!(read_source(root.path(), "deleted.rs").is_empty());
        assert_eq!(
            take_errors().first().map(|error| &error.kind),
            Some(&CodegraphErrorKind::FileDeletionRace)
        );

        begin_operation();
        assert!(read_source(root.path(), "never.rs").is_empty());
        assert_eq!(
            take_errors().first().map(|error| &error.kind),
            Some(&CodegraphErrorKind::SourceMissing)
        );
        Ok(())
    }
}
