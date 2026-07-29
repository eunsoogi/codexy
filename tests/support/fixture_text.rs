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

#[test]
fn text_fixture_normalization_preserves_raw_binary_reads() -> std::io::Result<()> {
    let temp = tempfile::tempdir()?;
    let text = temp.path().join("text.txt");
    let binary = temp.path().join("binary.bin");
    std::fs::write(&text, b"title\r\nbody\r\n")?;
    std::fs::write(&binary, [0_u8, b'\r', b'\n', 0xff])?;

    assert_eq!(read_text_fixture(&text)?, "title\nbody\n");
    assert_eq!(std::fs::read(&binary)?, [0_u8, b'\r', b'\n', 0xff]);
    Ok(())
}

#[cfg(unix)]
#[test]
fn materialized_text_fixture_keeps_executable_mode_and_canonical_lf() -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt as _;

    let temp = tempfile::tempdir()?;
    let source = temp.path().join("source.sh");
    let target = temp.path().join("nested/target.sh");
    std::fs::write(&source, "#!/bin/sh\r\nprintf fixture\r\n")?;
    std::fs::set_permissions(&source, std::fs::Permissions::from_mode(0o755))?;

    materialize_lf_text_fixture(&source, &target)?;

    assert_eq!(std::fs::read(&target)?, b"#!/bin/sh\nprintf fixture\n");
    assert_eq!(
        std::fs::metadata(&target)?.permissions().mode() & 0o111,
        0o111
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn materialized_script_fixture_keeps_shebang_siblings_available()
-> Result<(), Box<dyn std::error::Error>> {
    use std::os::unix::fs::PermissionsExt as _;

    let temp = tempfile::tempdir()?;
    let source_dir = temp.path().join("source");
    let target = temp.path().join("target/inspector.sh");
    std::fs::create_dir(&source_dir)?;
    let source = source_dir.join("inspector.sh");
    let helper = source_dir.join("helper.sh");
    std::fs::write(
        &source,
        "#!/bin/sh\r\nexec \"$(dirname \"$0\")/helper.sh\" \"$@\"\r\n",
    )?;
    std::fs::write(&helper, "#!/bin/sh\r\nprintf '%s\\n' \"$1\"\r\n")?;
    for path in [&source, &helper] {
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))?;
    }

    materialize_lf_text_fixture(&source, &target)?;
    let output = std::process::Command::new(&target)
        .arg("argv with spaces")
        .output()?;

    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout)?, "argv with spaces\n");
    assert!(
        target
            .parent()
            .expect("target parent")
            .join("helper.sh")
            .is_file()
    );
    Ok(())
}
