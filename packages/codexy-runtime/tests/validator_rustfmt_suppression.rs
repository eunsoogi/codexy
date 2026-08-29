use std::io;
use std::path::{Path, PathBuf};

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
    let compact: String = source
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect();
    if compact.contains(&format!("{}{}", FORMAT_NAMESPACE, SKIP_DIRECTIVE)) {
        Err(format!(
            "{} contains a formatter suppression",
            path.display()
        ))
    } else {
        Ok(())
    }
}
