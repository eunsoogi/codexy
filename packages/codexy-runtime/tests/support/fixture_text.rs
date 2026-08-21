use std::{
    collections::HashMap,
    io::Write,
    path::Path,
    sync::{Mutex, OnceLock},
};

pub(crate) fn write_fixture_with_permissions(
    path: &Path,
    source: impl AsRef<[u8]>,
    permissions: std::fs::Permissions,
) -> std::io::Result<()> {
    write_fixture_atomically(path, source.as_ref(), |staged| {
        std::fs::set_permissions(staged, permissions)
    })
}

pub(crate) fn write_fixture_atomically(
    path: &Path,
    source: &[u8],
    prepare: impl FnOnce(&Path) -> std::io::Result<()>,
) -> std::io::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| std::io::Error::other("fixture path must have a parent"))?;
    std::fs::create_dir_all(parent)?;
    let mut staged = tempfile::NamedTempFile::new_in(parent)?;
    staged.write_all(source)?;
    prepare(staged.path())?;
    staged.as_file().sync_all()?;
    staged
        .persist(path)
        .map_err(|error| error.error)
        .map(|_| ())
}

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
    let permissions = std::fs::metadata(source)?.permissions();
    write_fixture_with_permissions(target, read_text_fixture(source)?, permissions)
}
