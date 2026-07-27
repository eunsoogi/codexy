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
    let parent = target
        .parent()
        .ok_or_else(|| std::io::Error::other("fixture target must have a parent"))?;
    std::fs::create_dir_all(parent)?;
    std::fs::write(target, read_text_fixture(source)?)
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
