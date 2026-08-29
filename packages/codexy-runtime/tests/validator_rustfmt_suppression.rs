use std::io;
use std::path::{Path, PathBuf};

#[path = "validator_rustfmt_suppression/strings.rs"]
mod strings;
use strings::{quoted_string_end, raw_string_end, raw_string_start};

const FORMAT_NAMESPACE: &str = "rustfmt";
const SKIP_DIRECTIVE: &str = "::skip";
type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
#[test]
fn maintained_rust_rejects_formatter_suppressions() -> TestResult {
    let fixture = format!(
        "#[{}{}]\nfn fixture() {{}}\n",
        FORMAT_NAMESPACE, SKIP_DIRECTIVE
    );
    let error =
        check_source(Path::new("fixture.rs"), &fixture).expect_err("the fixture must be rejected");
    assert!(error.contains("formatter suppression"), "{error}");

    let spaced_introducer_fixture = format!(
        "# [{}{}]\nfn fixture() {{}}\n",
        FORMAT_NAMESPACE, SKIP_DIRECTIVE
    );
    assert!(
        check_source(
            Path::new("spaced-introducer-fixture.rs"),
            &spaced_introducer_fixture
        )
        .is_err(),
        "whitespace between # and [ must not evade the suppression check"
    );
    let commented_introducer_fixture = format!(
        "# /* sentinel */ [{}{}]\nfn fixture() {{}}\n",
        FORMAT_NAMESPACE, SKIP_DIRECTIVE
    );
    assert!(
        check_source(
            Path::new("commented-introducer-fixture.rs"),
            &commented_introducer_fixture
        )
        .is_err(),
        "comments between # and [ must not evade the suppression check"
    );

    let spaced_fixture = format!(
        "#[{} /* sentinel */ :: skip]\nfn fixture() {{}}\n",
        FORMAT_NAMESPACE
    );
    assert!(
        check_source(Path::new("spaced-fixture.rs"), &spaced_fixture).is_err(),
        "comments and whitespace must not evade the suppression check"
    );
    let string_fixture = format!("const FIXTURE: &str = {fixture:?};\n");
    assert!(
        check_source(Path::new("string-fixture.rs"), &string_fixture).is_ok(),
        "a string containing the marker must not be rejected"
    );
    let comment_fixture = format!("// {fixture}fn fixture() {{}}\n");
    assert!(
        check_source(Path::new("comment-fixture.rs"), &comment_fixture).is_ok(),
        "a comment containing the marker must not be rejected"
    );

    let runtime_root = codexy_runtime::paths::runtime_package_root();
    let mut files = Vec::new();
    collect_rust_files(&runtime_root.join("src"), &mut files)?;
    collect_rust_files(&runtime_root.join("tests"), &mut files)?;
    files.sort();
    assert!(!files.is_empty(), "maintained Rust source must be present");
    for path in files {
        let source = std::fs::read_to_string(&path)?;
        check_source(&path, &source).map_err(io::Error::other)?;
    }
    Ok(())
}

fn collect_rust_files(root: &Path, files: &mut Vec<PathBuf>) -> io::Result<()> {
    for entry in std::fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if entry.file_type()?.is_dir() {
            collect_rust_files(&path, files)?;
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
        }
    }
    Ok(())
}

fn check_source(path: &Path, source: &str) -> Result<(), String> {
    if !contains_formatter_suppression(source) {
        return Ok(());
    }
    Err(format!(
        "{} contains a formatter suppression",
        path.display()
    ))
}

fn contains_formatter_suppression(source: &str) -> bool {
    let bytes = source.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if let Some(next) = skip_ignored(bytes, index) {
            index = next;
            continue;
        }
        if bytes[index] == b'#' {
            if let Some(opening_bracket) = outer_attribute_open(bytes, index) {
                if let Some(end) = attribute_end(bytes, opening_bracket) {
                    if attribute_contains_suppression(&source[opening_bracket + 1..end]) {
                        return true;
                    }
                    index = end + 1;
                    continue;
                }
            }
        }
        index += 1;
    }
    false
}

fn outer_attribute_open(bytes: &[u8], hash: usize) -> Option<usize> {
    let mut index = hash + 1;
    while index < bytes.len() {
        if bytes[index].is_ascii_whitespace() {
            index += 1;
        } else if bytes[index] == b'/'
            && (bytes.get(index + 1) == Some(&b'/') || bytes.get(index + 1) == Some(&b'*'))
        {
            index = skip_ignored(bytes, index)?;
        } else {
            return (bytes[index] == b'[').then_some(index);
        }
    }
    None
}

fn attribute_end(bytes: &[u8], opening_bracket: usize) -> Option<usize> {
    let mut depth = 0;
    let mut index = opening_bracket;
    while index < bytes.len() {
        if let Some(next) = skip_ignored(bytes, index) {
            index = next;
            continue;
        }
        match bytes[index] {
            b'[' => depth += 1,
            b']' if depth == 1 => return Some(index),
            b']' => depth -= 1,
            _ => {}
        }
        index += 1;
    }
    None
}

fn attribute_contains_suppression(body: &str) -> bool {
    let bytes = body.as_bytes();
    let mut index = 0;
    let mut state = 0;
    while index < bytes.len() {
        if let Some(next) = skip_ignored(bytes, index) {
            index = next;
            continue;
        }
        if bytes[index].is_ascii_whitespace() {
            index += 1;
            continue;
        }
        if bytes[index] == b'_' || bytes[index].is_ascii_alphabetic() {
            let start = index;
            index += 1;
            while index < bytes.len() && is_identifier_continue(bytes[index]) {
                index += 1;
            }
            let identifier = &bytes[start..index];
            if state == 2 && identifier == b"skip" {
                return true;
            }
            state = if identifier == b"rustfmt" { 1 } else { 0 };
            continue;
        }
        if bytes[index] == b':' && bytes.get(index + 1) == Some(&b':') {
            state = if state == 1 { 2 } else { 0 };
            index += 2;
            continue;
        }
        state = 0;
        index += 1;
    }
    false
}

fn skip_ignored(bytes: &[u8], index: usize) -> Option<usize> {
    if bytes.get(index) == Some(&b'/') && bytes.get(index + 1) == Some(&b'/') {
        let mut cursor = index + 2;
        while cursor < bytes.len() && bytes[cursor] != b'\n' {
            cursor += 1;
        }
        return Some(cursor);
    }
    if bytes.get(index) == Some(&b'/') && bytes.get(index + 1) == Some(&b'*') {
        let mut depth = 1;
        let mut cursor = index + 2;
        while cursor + 1 < bytes.len() {
            if bytes[cursor] == b'/' && bytes[cursor + 1] == b'*' {
                depth += 1;
                cursor += 2;
            } else if bytes[cursor] == b'*' && bytes[cursor + 1] == b'/' {
                depth -= 1;
                cursor += 2;
                if depth == 0 {
                    return Some(cursor);
                }
            } else {
                cursor += 1;
            }
        }
        return Some(bytes.len());
    }
    if let Some((content_start, hashes)) = raw_string_start(bytes, index) {
        return Some(raw_string_end(bytes, content_start, hashes));
    }
    if bytes.get(index) == Some(&b'b') && bytes.get(index + 1) == Some(&b'"') {
        return Some(quoted_string_end(bytes, index + 1));
    }
    if bytes.get(index) == Some(&b'"') {
        return Some(quoted_string_end(bytes, index));
    }
    None
}

fn is_identifier_continue(byte: u8) -> bool {
    byte == b'_' || byte.is_ascii_alphanumeric()
}
