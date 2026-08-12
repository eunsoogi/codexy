use std::path::{Path, PathBuf};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn all_packaged_skill_local_links_resolve() -> TestResult {
    let root = codexy_runtime::paths::repository_root();
    for path in skill_markdown_files(&root)? {
        assert_local_links(&path, &std::fs::read_to_string(&path)?)?;
    }
    Ok(())
}

#[test]
fn all_packaged_skill_markdown_is_collected() -> TestResult {
    let root = codexy_runtime::paths::repository_root();
    let markdown = skill_markdown_files(&root)?;
    for expected in [
        root.join("plugins/codexy/skills/engineering/SKILL.md"),
        root.join("plugins/codexy-github/skills/git-workflow/references/local-git-and-branches.md"),
    ] {
        assert!(markdown.contains(&expected), "missing {}", expected.display());
    }
    Ok(())
}

#[test]
fn packaged_skill_local_links_reject_missing_targets() -> TestResult {
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

fn skill_markdown_files(root: &Path) -> Result<Vec<PathBuf>, std::io::Error> {
    let mut files = Vec::new();
    collect_markdown_files(&root.join("plugins/codexy/skills"), &mut files)?;
    collect_markdown_files(&root.join("plugins/codexy-github/skills"), &mut files)?;
    files.sort();
    Ok(files)
}

fn collect_markdown_files(root: &Path, files: &mut Vec<PathBuf>) -> Result<(), std::io::Error> {
    for entry in std::fs::read_dir(root)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_markdown_files(&path, files)?;
        } else if path.extension().is_some_and(|extension| extension == "md") {
            files.push(path);
        }
    }
    Ok(())
}
