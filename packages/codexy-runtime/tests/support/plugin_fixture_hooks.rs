use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};
use std::time::Instant;

use serde_json::Value;

pub(super) fn copy(
    source: &Path,
    target: &Path,
    mutable_files: &[&Path],
    identity: &str,
) -> std::io::Result<()> {
    let files = files_to_copy(source, mutable_files)?;
    materialize_files(source, target, &files, mutable_files, identity)?;
    for relative in mutable_files {
        if source.join(relative).is_file() {
            continue;
        }
        let target_path = target.join(relative);
        target_path
            .parent()
            .ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "hook fixture mutable path must have a parent directory",
                )
            })
            .and_then(std::fs::create_dir_all)?;
    }
    if source.join("hooks/capability-contract.json").is_file() {
        super::materialize_admission_runtime_suite(target)?;
    }
    Ok(())
}

pub(super) fn validate_mutable_files(
    source: &Path,
    mutable_files: &[&Path],
) -> std::io::Result<()> {
    for relative in mutable_files {
        if !relative.is_relative()
            || relative.as_os_str().is_empty()
            || relative
                .components()
                .any(|component| !matches!(component, Component::Normal(_)))
        {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "hook fixture mutable path must be a relative regular file",
            ));
        }
        let path = source.join(relative);
        if path.exists() && !path.is_file() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "hook fixture mutable path must name a source file or a new file",
            ));
        }
    }
    Ok(())
}

fn materialize_files(
    source: &Path,
    target: &Path,
    files: &[PathBuf],
    mutable_files: &[&Path],
    identity: &str,
) -> std::io::Result<()> {
    #[cfg(not(windows))]
    let _ = mutable_files;
    let profiling = super::super::profile_metrics::enabled();
    let started = Instant::now();
    let mut copied_files = 0;
    let mut copied_bytes = 0;
    for relative in files {
        let source_path = source.join(relative);
        let target_path = target.join(relative);
        target_path
            .parent()
            .ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "hook fixture file must have a parent directory",
                )
            })
            .and_then(std::fs::create_dir_all)?;
        copy_seed_file(&source_path, &target_path)?;
        #[cfg(windows)]
        set_readonly(
            &target_path,
            !mutable_files.iter().any(|path| {
                super::super::plugin_fixture_mutable::normalized(path)
                    == super::super::plugin_fixture_mutable::normalized(relative)
            }),
        )?;
        if profiling {
            copied_files += 1;
            copied_bytes += std::fs::metadata(source_path)?.len();
        }
    }
    if profiling {
        super::super::profile_metrics::record_fixture_materialization(
            identity,
            copied_files,
            copied_bytes,
            started.elapsed().as_secs_f64(),
        );
    }
    Ok(())
}

fn files_to_copy(source: &Path, mutable_files: &[&Path]) -> std::io::Result<Vec<PathBuf>> {
    let mut files = BTreeSet::new();
    files.insert(PathBuf::from(".codex-plugin/plugin.json"));
    files.insert(PathBuf::from("hooks/hooks.json"));
    if source.join("hooks/capability-contract.json").is_file() {
        files.insert(PathBuf::from("hooks/capability-contract.json"));
        let hooks_path = source.join("hooks/hooks.json");
        let hooks = std::fs::read_to_string(&hooks_path)?;
        let hooks: Value = serde_json::from_str(&hooks).map_err(|error| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("invalid hook fixture configuration: {error}"),
            )
        })?;
        collect_entrypoints(&hooks, &mut files);
    } else {
        collect_hook_tree(source, &mut files)?;
    }
    files.extend(
        mutable_files
            .iter()
            .filter(|path| source.join(path).is_file())
            .map(|path| (*path).to_path_buf()),
    );
    Ok(files.into_iter().collect())
}

fn collect_entrypoints(value: &Value, files: &mut BTreeSet<PathBuf>) {
    match value {
        Value::Object(object) => {
            for (key, value) in object {
                if matches!(key.as_str(), "command" | "commandWindows") {
                    if let Some(command) = value.as_str() {
                        if let Some(path) = entrypoint_path(command) {
                            files.insert(path);
                        }
                    }
                }
                collect_entrypoints(value, files);
            }
        }
        Value::Array(values) => {
            for value in values {
                collect_entrypoints(value, files);
            }
        }
        _ => {}
    }
}

fn entrypoint_path(command: &str) -> Option<PathBuf> {
    let command = command.trim_start();
    let entrypoint = if let Some(command) = command.strip_prefix('"') {
        command.split_once('"')?.0
    } else {
        command.split_whitespace().next()?
    };
    for marker in ["${PLUGIN_ROOT}/", "$PLUGIN_ROOT/"] {
        let Some(path) = entrypoint.strip_prefix(marker) else {
            continue;
        };
        let path = Path::new(path);
        if !path.as_os_str().is_empty()
            && path.is_relative()
            && path
                .components()
                .all(|component| matches!(component, Component::Normal(_)))
        {
            return Some(path.to_path_buf());
        }
    }
    None
}

fn collect_hook_tree(source: &Path, files: &mut BTreeSet<PathBuf>) -> std::io::Result<()> {
    let hooks = source.join("hooks");
    collect_hook_tree_at(&hooks, Path::new("hooks"), files)
}

fn collect_hook_tree_at(
    source: &Path,
    relative: &Path,
    files: &mut BTreeSet<PathBuf>,
) -> std::io::Result<()> {
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let entry_relative = relative.join(entry.file_name());
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            if entry.file_name() == "__pycache__" {
                continue;
            }
            collect_hook_tree_at(&source_path, &entry_relative, files)?;
        } else if file_type.is_file() {
            files.insert(entry_relative);
        }
    }
    Ok(())
}

fn copy_seed_file(source: &Path, target: &Path) -> std::io::Result<()> {
    super::super::profile_metrics::record("fixture_copy_file");
    std::fs::copy(source, target).map(|_| ())
}

#[cfg(windows)]
fn set_readonly(path: &Path, readonly: bool) -> std::io::Result<()> {
    let mut permissions = std::fs::metadata(path)?.permissions();
    permissions.set_readonly(readonly);
    std::fs::set_permissions(path, permissions)
}
