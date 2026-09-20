use std::fs;
use std::path::Path;
use std::process::Command;

use super::git::collect_working_tree_with_hook;
use super::{ChangeKind, ChangeScope, FileObservedState, ObservedState, WorkingTreeScope, collect};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn commit_comparison_is_repeatable_and_preserves_file_changes() -> TestResult {
    let repository = initialized_repository()?;
    let root = repository.path();
    fs::write(root.join("modified.rs"), "before\n")?;
    run_git(root, ["add", "modified.rs"])?;
    fs::write(root.join("delete me.rs"), "delete\n")?;
    run_git(root, ["add", "delete me.rs"])?;
    run_git(root, ["commit", "--quiet", "-m", "add deletion target"])?;
    let base = run_git(root, ["rev-parse", "HEAD"])?;
    fs::write(root.join("modified.rs"), "after\n")?;
    fs::remove_file(root.join("delete me.rs"))?;
    run_git(root, ["mv", "tracked.rs", "renamed file.rs"])?;
    fs::write(root.join("added file.rs"), "added\n")?;
    run_git(root, ["add", "-A"])?;
    run_git(root, ["commit", "--quiet", "-m", "change files"])?;
    let head = run_git(root, ["rev-parse", "HEAD"])?;

    let first = collect(root, ChangeScope::head_comparison(&base, &head))?;
    let second = collect(root, ChangeScope::head_comparison(&base, &head))?;
    assert_eq!(
        first, second,
        "the same repository state must sort identically"
    );
    assert_eq!(first.baseline_revision, base);
    assert!(matches!(
        first.observed_state,
        ObservedState::HeadComparison { .. }
    ));
    assert!(first.changes.iter().any(|change| {
        change.kind == ChangeKind::Added && change.current_path.as_deref() == Some("added file.rs")
    }));
    assert!(first.changes.iter().any(|change| {
        change.kind == ChangeKind::Deleted
            && change.previous_path.as_deref() == Some("delete me.rs")
    }));
    assert!(first.changes.iter().any(|change| {
        change.kind == ChangeKind::Modified && change.current_path.as_deref() == Some("modified.rs")
    }));
    assert!(first.changes.iter().any(|change| {
        change.kind == ChangeKind::Renamed
            && change.previous_path.as_deref() == Some("tracked.rs")
            && change.current_path.as_deref() == Some("renamed file.rs")
    }));
    assert!(
        first
            .changes
            .iter()
            .all(|change| matches!(change.observed_state, FileObservedState::HeadComparison))
    );
    Ok(())
}

#[test]
fn comparison_treats_git_type_changes_as_modified() -> TestResult {
    let changes = super::parse::parse_diff(b"T\0changed.rs\0")?;
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].kind, ChangeKind::Modified);
    assert_eq!(changes[0].current_path.as_deref(), Some("changed.rs"));
    Ok(())
}

#[test]
fn working_tree_reports_staged_unstaged_and_optional_untracked_scopes() -> TestResult {
    let repository = initialized_repository()?;
    let root = repository.path();
    fs::write(root.join("delete me.rs"), "delete\n")?;
    run_git(root, ["add", "delete me.rs"])?;
    run_git(root, ["commit", "--quiet", "-m", "add deletion target"])?;
    fs::write(root.join("mixed.rs"), "staged\n")?;
    run_git(root, ["add", "mixed.rs"])?;
    fs::write(root.join("mixed.rs"), "unstaged\n")?;
    run_git(root, ["mv", "tracked.rs", "renamed name.rs"])?;
    fs::remove_file(root.join("delete me.rs"))?;
    fs::write(root.join("untracked name.rs"), "untracked\n")?;

    let included = collect(root, ChangeScope::working_tree(true))?;
    assert!(matches!(
        included.observed_state,
        ObservedState::WorkingTree { .. }
    ));
    let mixed = included
        .changes
        .iter()
        .find(|change| change.current_path.as_deref() == Some("mixed.rs"))
        .ok_or("missing mixed tracked change")?;
    assert_eq!(mixed.kind, ChangeKind::Added);
    assert_eq!(
        mixed.observed_state,
        FileObservedState::WorkingTree {
            scopes: vec![WorkingTreeScope::Staged, WorkingTreeScope::Unstaged]
        }
    );
    let renamed = included
        .changes
        .iter()
        .find(|change| change.current_path.as_deref() == Some("renamed name.rs"))
        .ok_or("missing working-tree rename")?;
    assert_eq!(renamed.kind, ChangeKind::Renamed);
    assert_eq!(renamed.previous_path.as_deref(), Some("tracked.rs"));
    assert_eq!(
        renamed.observed_state,
        FileObservedState::WorkingTree {
            scopes: vec![WorkingTreeScope::Staged]
        }
    );
    assert!(included.changes.iter().any(|change| {
        change.kind == ChangeKind::Deleted
            && change.previous_path.as_deref() == Some("delete me.rs")
    }));
    let untracked = included
        .changes
        .iter()
        .find(|change| change.current_path.as_deref() == Some("untracked name.rs"))
        .ok_or("missing untracked change")?;
    assert_eq!(untracked.kind, ChangeKind::Added);
    assert_eq!(
        untracked.observed_state,
        FileObservedState::WorkingTree {
            scopes: vec![WorkingTreeScope::Untracked]
        }
    );

    let excluded = collect(root, ChangeScope::working_tree(false))?;
    assert!(
        !excluded
            .changes
            .iter()
            .any(|change| change.current_path.as_deref() == Some("untracked name.rs"))
    );
    assert!(matches!(
        excluded.observed_state,
        ObservedState::WorkingTree { .. }
    ));
    Ok(())
}

#[test]
fn invalid_revisions_and_non_root_paths_fail_without_collecting() -> TestResult {
    let repository = initialized_repository()?;
    let root = repository.path();
    let invalid = collect(root, ChangeScope::head_comparison("missing-ref", "HEAD"))
        .expect_err("an invalid base ref must fail");
    assert!(invalid.to_string().contains("rev-parse"));

    let nested = root.join("nested");
    fs::create_dir(&nested)?;
    let non_root = collect(&nested, ChangeScope::working_tree(false))
        .expect_err("a subdirectory is not an explicit repository root");
    assert!(non_root.to_string().contains("worktree root"));

    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        let outside = tempfile::tempdir()?;
        fs::create_dir(root.join("linked"))?;
        fs::write(root.join("linked/tracked.rs"), "tracked\n")?;
        run_git(root, ["add", "linked/tracked.rs"])?;
        run_git(root, ["commit", "--quiet", "-m", "add linked file"])?;
        fs::remove_dir_all(root.join("linked"))?;
        symlink(outside.path(), root.join("linked"))?;
        let error = collect(root, ChangeScope::working_tree(true))
            .expect_err("a symlinked ancestor must not be read");
        assert!(error.to_string().contains("symlinked ancestor"));
    }
    Ok(())
}

#[test]
fn changing_worktree_state_during_collection_is_reported() -> TestResult {
    let repository = initialized_repository()?;
    let path = repository.path().join("tracked.rs");
    let result = collect_working_tree_with_hook(repository.path(), true, || {
        fs::write(&path, "changed during collection\n").expect("test write");
    });
    let error = result.expect_err("a changed snapshot must not be returned as stable");
    assert!(error.to_string().contains("changed during collection"));
    Ok(())
}

#[test]
fn changing_already_dirty_content_during_collection_is_reported() -> TestResult {
    let repository = initialized_repository()?;
    let root = repository.path();
    let tracked = root.join("tracked.rs");
    fs::write(&tracked, "dirty before\n")?;
    let tracked_result = collect_working_tree_with_hook(root, true, || {
        fs::write(&tracked, "dirty after\n").expect("test write");
    });
    assert!(
        tracked_result
            .expect_err("a dirty tracked file content change must be detected")
            .to_string()
            .contains("changed during collection")
    );

    let untracked = root.join("untracked.rs");
    fs::write(&untracked, "untracked before\n")?;
    let untracked_result = collect_working_tree_with_hook(root, true, || {
        fs::write(&untracked, "untracked after\n").expect("test write");
    });
    assert!(
        untracked_result
            .expect_err("an untracked file content change must be detected")
            .to_string()
            .contains("changed during collection")
    );
    Ok(())
}

fn initialized_repository() -> TestResult<tempfile::TempDir> {
    let repository = tempfile::tempdir()?;
    run_git(repository.path(), ["init", "--quiet"])?;
    run_git(
        repository.path(),
        ["config", "user.email", "tests@example.invalid"],
    )?;
    run_git(repository.path(), ["config", "user.name", "Codexy Tests"])?;
    fs::write(repository.path().join("tracked.rs"), "initial\n")?;
    run_git(repository.path(), ["add", "tracked.rs"])?;
    run_git(repository.path(), ["commit", "--quiet", "-m", "initial"])?;
    Ok(repository)
}

fn run_git<const N: usize>(root: &Path, args: [&str; N]) -> TestResult<String> {
    let output = Command::new("git").args(args).current_dir(root).output()?;
    if !output.status.success() {
        return Err(format!(
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}
