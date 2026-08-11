use std::path::Path;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn local_orchestration_reference_links_resolve() -> TestResult {
    let root = codexy_runtime::paths::repository_root();
    let references = root.join("plugins/codexy/skills/orchestration/references");
    for entry in std::fs::read_dir(references)? {
        let path = entry?.path();
        if path.extension().is_some_and(|extension| extension == "md") {
            assert_local_links(&path, &std::fs::read_to_string(&path)?)?;
        }
    }
    Ok(())
}

#[test]
fn local_orchestration_reference_links_reject_missing_targets() -> TestResult {
    let root = codexy_runtime::paths::repository_root();
    let path = root.join("plugins/codexy/skills/orchestration/references/token-efficient.md");

    assert!(assert_local_links(&path, "[missing](not-a-real-file.md)").is_err());
    Ok(())
}

fn assert_local_links(path: &Path, text: &str) -> Result<(), String> {
    let base = path.parent().ok_or("document has no parent")?;
    for remainder in text.split("](").skip(1) {
        let target = remainder.split(')').next().ok_or("unterminated link")?;
        if target.starts_with("http://") || target.starts_with("https://") || target.starts_with('#') {
            continue;
        }
        let local = target.split('#').next().unwrap_or(target);
        if !base.join(local).exists() {
            return Err(format!("broken local link from {} to {target}", path.display()));
        }
    }
    Ok(())
}
