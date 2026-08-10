type TestResult = Result<(), Box<dyn std::error::Error>>;

use std::{fs, ops::Deref, path::{Path, PathBuf}};

use crate::support::wiki_migration_fixture::assert_successful_additive_migration;

#[test]
fn successful_migration_rejects_each_unauthorized_delta() -> TestResult {
    for (name, path, from, to) in [
        ("title", "wiki/topic.md", "Legacy topic", "Changed topic"),
        ("source", "wiki/topic.md", "raw/source.md", "raw/other.md"),
        ("verified", "wiki/topic.md", "2026-08-09", "2026-08-08"),
        ("volatility", "wiki/topic.md", "warm", "cold"),
        ("confidence", "wiki/topic.md", "medium", "high"),
        ("updated", "wiki/topic.md", "updated: 2026-08-09", "updated: 2026-08-08"),
        (
            "unknown frontmatter",
            "wiki/topic.md",
            "confidence: medium",
            "confidence: medium\nunsupported: value",
        ),
        (
            "body",
            "wiki/topic.md",
            "Derived metadata was added without modifying raw history.",
            "Unauthorized body rewrite.",
        ),
    ] {
        let success = copied_success()?;
        replace(&success.join("after").join(path), from, to)?;
        assert!(assert_successful_additive_migration(&success).is_err(), "{name}");
    }
    for (name, change) in [
        ("blank log line", "\n"),
        ("second log entry", "\nsecond migration entry"),
        ("unknown log field", "\nunknown=value"),
    ] {
        let success = copied_success()?;
        let log = success.join("after/log.md");
        fs::write(&log, format!("{}{}", fs::read_to_string(&log)?, change))?;
        assert!(assert_successful_additive_migration(&success).is_err(), "{name}");
    }
    Ok(())
}

#[test]
fn successful_migration_rejects_path_and_log_shape_mutations() -> TestResult {
    let success = copied_success()?;
    fs::remove_file(success.join("after/wiki/topic.md"))?;
    assert!(assert_successful_additive_migration(&success).is_err(), "missing path");

    let success = copied_success()?;
    fs::rename(success.join("after/raw/source.md"), success.join("after/raw/renamed.md"))?;
    assert!(assert_successful_additive_migration(&success).is_err(), "renamed path");

    let success = copied_success()?;
    fs::write(success.join("after/extra.md"), "unexpected")?;
    assert!(assert_successful_additive_migration(&success).is_err(), "extra path");

    let success = copied_success()?;
    fs::write(success.join("after/raw/source.md"), "mutated raw history")?;
    assert!(assert_successful_additive_migration(&success).is_err(), "raw mutation");

    for (name, from, to) in [
        ("malformed log entry", " migration ", " invalid "),
        ("wrong log field", "bytes=196", "bytes=1"),
        ("wrong freshness", "freshness=valid", "freshness=unknown"),
    ] {
        let success = copied_success()?;
        replace(&success.join("after/log.md"), from, to)?;
        assert!(assert_successful_additive_migration(&success).is_err(), "{name}");
    }
    let success = copied_success()?;
    fs::copy(success.join("before/log.md"), success.join("after/log.md"))?;
    assert!(assert_successful_additive_migration(&success).is_err(), "no append");
    Ok(())
}

#[test]
fn copied_success_fixture_is_removed_after_scope() -> TestResult {
    let retained = {
        let success = copied_success()?;
        success.to_path_buf()
    };
    assert!(!retained.exists(), "temporary success fixture leaked: {}", retained.display());
    Ok(())
}

struct CopiedSuccess {
    _temporary_root: tempfile::TempDir,
    root: PathBuf,
}

impl Deref for CopiedSuccess {
    type Target = Path;

    fn deref(&self) -> &Self::Target {
        &self.root
    }
}

fn copied_success() -> Result<CopiedSuccess, Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let destination = temp.path().join("success");
    let source = codexy_runtime::paths::repository_root()
        .join("packages/codexy-runtime/tests/fixtures/wiki-core/migration/success");
    copy_dir(&source, &destination)?;
    Ok(CopiedSuccess {
        _temporary_root: temp,
        root: destination,
    })
}

fn replace(path: &Path, from: &str, to: &str) -> Result<(), Box<dyn std::error::Error>> {
    let source = fs::read_to_string(path)?;
    let target = source.replacen(from, to, 1);
    (source != target).then_some(()).ok_or("fixture identity missing")?;
    fs::write(path, target)?;
    Ok(())
}

fn copy_dir(source: &Path, destination: &Path) -> Result<(), std::io::Error> {
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let target = destination.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}
