use std::{
    collections::HashMap,
    sync::{Mutex, OnceLock},
};

static MATERIALIZED_SCRIPT_SOURCES: OnceLock<
    Mutex<HashMap<std::path::PathBuf, std::path::PathBuf>>,
> = OnceLock::new();

pub(crate) fn normalize_fixture_text(text: &str) -> String {
    text.replace("\r\n", "\n")
}

pub(crate) fn read_text_fixture(path: &std::path::Path) -> std::io::Result<String> {
    let text = std::fs::read_to_string(path)?;
    Ok(normalize_fixture_text(&text))
}

pub(crate) fn materialize_lf_text_fixture(
    source: &std::path::Path,
    target: &std::path::Path,
) -> std::io::Result<()> {
    materialize_lf_text_fixture_file(source, target)?;
    if read_text_fixture(source)?.starts_with("#!") {
        let source_dir = source
            .parent()
            .ok_or_else(|| std::io::Error::other("fixture source must have a parent"))?;
        let target_dir = target
            .parent()
            .ok_or_else(|| std::io::Error::other("fixture target must have a parent"))?;
        for entry in std::fs::read_dir(source_dir)? {
            let entry = entry?;
            let sibling = entry.path();
            if sibling != source && sibling.is_file() && std::fs::read(&sibling)?.starts_with(b"#!")
            {
                materialize_lf_text_fixture_file(&sibling, &target_dir.join(entry.file_name()))?;
            }
        }
        record_materialized_script_source(source, target);
    }
    Ok(())
}

pub(crate) fn materialized_script_source(path: &std::path::Path) -> Option<std::path::PathBuf> {
    MATERIALIZED_SCRIPT_SOURCES
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .ok()?
        .get(&fixture_path_key(path))
        .cloned()
}

fn record_materialized_script_source(source: &std::path::Path, target: &std::path::Path) {
    if let Ok(mut sources) = MATERIALIZED_SCRIPT_SOURCES
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
    {
        sources.insert(fixture_path_key(target), fixture_path_key(source));
    }
}

fn fixture_path_key(path: &std::path::Path) -> std::path::PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

fn materialize_lf_text_fixture_file(
    source: &std::path::Path,
    target: &std::path::Path,
) -> std::io::Result<()> {
    let parent = target
        .parent()
        .ok_or_else(|| std::io::Error::other("fixture target must have a parent"))?;
    std::fs::create_dir_all(parent)?;
    std::fs::write(target, read_text_fixture(source)?)?;
    std::fs::set_permissions(target, std::fs::metadata(source)?.permissions())
}
