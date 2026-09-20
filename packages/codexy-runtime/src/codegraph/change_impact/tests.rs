#[path = "fixtures.rs"]
mod fixtures;

use std::fs;

use anyhow::Result;

use super::super::change_input::ChangeKind;
use super::{ImpactLevel, ImpactOptions, UnknownReason, analyze, analyze_with_options};
use fixtures::repository;

#[test]
fn python_changes_include_direct_and_transitive_dependents() -> Result<()> {
    let fixture = repository(&[
        ("dep.py", "VALUE = 1\n"),
        ("mid.py", "import dep\n"),
        ("app.py", "import mid\n"),
    ])?;
    let base = fixture.head()?;
    fixture.write("dep.py", "VALUE = 2\n")?;
    let head = fixture.commit("change dependency")?;

    let result = analyze(fixture.path(), &fixture.comparison(&base, &head)?)?;

    assert_eq!(impact(&result, "dep.py"), Some(ImpactLevel::Direct));
    assert_eq!(impact(&result, "mid.py"), Some(ImpactLevel::Transitive));
    assert_eq!(impact(&result, "app.py"), Some(ImpactLevel::Transitive));
    assert!(
        result
            .connecting_paths
            .iter()
            .any(|path| path.path == ["dep.py", "mid.py", "app.py"])
    );
    Ok(())
}

#[test]
fn rust_shared_modules_affect_each_importer() -> Result<()> {
    let fixture = repository(&[
        ("shared.rs", "pub const VALUE: u8 = 1;\n"),
        ("one.rs", "mod shared;\n"),
        ("two.rs", "mod shared;\n"),
    ])?;
    let base = fixture.head()?;
    fixture.write("shared.rs", "pub const VALUE: u8 = 2;\n")?;
    let head = fixture.commit("change shared module")?;

    let result = analyze(fixture.path(), &fixture.comparison(&base, &head)?)?;

    assert_eq!(impact(&result, "shared.rs"), Some(ImpactLevel::Direct));
    assert_eq!(impact(&result, "one.rs"), Some(ImpactLevel::Transitive));
    assert_eq!(impact(&result, "two.rs"), Some(ImpactLevel::Transitive));
    Ok(())
}

#[test]
fn cycles_terminate_and_keep_the_cycle_member_affected() -> Result<()> {
    let fixture = repository(&[("a.py", "import b\n"), ("b.py", "import a\n")])?;
    let base = fixture.head()?;
    fixture.write("a.py", "import b\nVALUE = 1\n")?;
    let head = fixture.commit("change cycle member")?;

    let result = analyze(fixture.path(), &fixture.comparison(&base, &head)?)?;

    assert_eq!(impact(&result, "a.py"), Some(ImpactLevel::Direct));
    assert_eq!(impact(&result, "b.py"), Some(ImpactLevel::Transitive));
    assert!(result.connecting_paths.len() <= 4);
    Ok(())
}

#[test]
fn deletions_and_renames_retain_previous_and_current_paths() -> Result<()> {
    let fixture = repository(&[("old.py", "VALUE = 1\n"), ("consumer.py", "import old\n")])?;
    let base = fixture.head()?;
    fs::rename(fixture.path().join("old.py"), fixture.path().join("new.py"))?;
    let head = fixture.commit("rename dependency")?;
    let changes = fixture.comparison(&base, &head)?;

    assert!(changes.changes.iter().any(|change| {
        change.kind == ChangeKind::Renamed
            && change.previous_path.as_deref() == Some("old.py")
            && change.current_path.as_deref() == Some("new.py")
    }));
    let result = analyze(fixture.path(), &changes)?;

    assert_eq!(impact(&result, "old.py"), Some(ImpactLevel::Direct));
    assert_eq!(impact(&result, "new.py"), Some(ImpactLevel::Direct));
    assert_eq!(
        impact(&result, "consumer.py"),
        Some(ImpactLevel::Transitive)
    );
    Ok(())
}

#[test]
fn deletions_keep_baseline_dependents() -> Result<()> {
    let fixture = repository(&[
        ("deleted.py", "VALUE = 1\n"),
        ("consumer.py", "import deleted\n"),
    ])?;
    let base = fixture.head()?;
    fs::remove_file(fixture.path().join("deleted.py"))?;
    let head = fixture.commit("delete dependency")?;

    let result = analyze(fixture.path(), &fixture.comparison(&base, &head)?)?;

    assert_eq!(impact(&result, "deleted.py"), Some(ImpactLevel::Direct));
    assert_eq!(
        impact(&result, "consumer.py"),
        Some(ImpactLevel::Transitive)
    );
    Ok(())
}

#[test]
fn unresolved_imports_and_unsupported_languages_are_unknown() -> Result<()> {
    let fixture = repository(&[
        ("entry.py", "VALUE = 1\n"),
        ("script.js", "export const A = 1;\n"),
    ])?;
    fixture.write("entry.py", "import missing\nVALUE = 2\n")?;
    fixture.write("script.js", "export const A = 2;\n")?;
    let result = analyze(fixture.path(), &fixture.working_tree()?)?;

    assert_eq!(impact(&result, "entry.py"), Some(ImpactLevel::Unknown));
    assert_eq!(impact(&result, "script.js"), Some(ImpactLevel::Unknown));
    assert!(
        result
            .limits
            .unknown
            .iter()
            .any(|area| area.reason == UnknownReason::UnresolvedImport)
    );
    assert!(
        result
            .limits
            .unknown
            .iter()
            .any(|area| area.reason == UnknownReason::UnsupportedLanguage)
    );
    assert!(result.partial);
    Ok(())
}

#[test]
fn parse_failures_are_preserved_as_unknown() -> Result<()> {
    let fixture = repository(&[("bad.py", "VALUE = 1\n")])?;
    fixture.write("bad.py", [0xff, 0xfe, b'\n'])?;
    let result = analyze(fixture.path(), &fixture.working_tree()?)?;

    assert_eq!(impact(&result, "bad.py"), Some(ImpactLevel::Unknown));
    assert!(
        result
            .limits
            .unknown
            .iter()
            .any(|area| area.reason == UnknownReason::ParseFailure)
    );
    Ok(())
}

#[test]
fn file_and_path_limits_are_explicit() -> Result<()> {
    let fixture = repository(&[
        ("dep.py", "VALUE = 1\n"),
        ("one.py", "import dep\n"),
        ("two.py", "import dep\n"),
    ])?;
    let base = fixture.head()?;
    fixture.write("dep.py", "VALUE = 2\n")?;
    let head = fixture.commit("change dependency")?;
    let changes = fixture.comparison(&base, &head)?;

    let file_limited = analyze_with_options(
        fixture.path(),
        &changes,
        ImpactOptions {
            max_files: 1,
            max_paths: 10,
        },
    )?;
    assert!(file_limited.limits.baseline_truncated);
    assert!(file_limited.limits.current_truncated);
    assert!(
        file_limited
            .limits
            .unknown
            .iter()
            .any(|area| area.reason == UnknownReason::FileLimit)
    );

    let path_limited = analyze_with_options(
        fixture.path(),
        &changes,
        ImpactOptions {
            max_files: 10,
            max_paths: 1,
        },
    )?;
    assert!(path_limited.limits.paths_truncated);
    assert!(
        path_limited
            .limits
            .unknown
            .iter()
            .any(|area| area.reason == UnknownReason::PathLimit)
    );
    Ok(())
}

fn impact(result: &super::ImpactAnalysis, path: &str) -> Option<ImpactLevel> {
    result
        .affected_files
        .iter()
        .find(|file| file.path == path)
        .map(|file| file.impact)
}
