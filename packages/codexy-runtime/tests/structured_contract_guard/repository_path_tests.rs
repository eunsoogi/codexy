use super::{
    repository_tests::git_fixture,
    structured_contract_guard::{comparison_counts_at, repository_violations_at},
};

#[test]
fn nested_and_noncanonical_package_roots_use_the_git_repository_prefix() {
    let fixture = git_fixture();
    fixture.write(
        "packages/runtime/tests/guarded.rs",
        "let skill = std::fs::read_to_string(\"plugins/codexy/skills/demo/SKILL.md\")?;\nassert!(skill.contains(\"heading\"));\n",
    );
    fixture.commit_all("base nested assertion");
    fixture.git(&["branch", "-M", "main"]);
    fixture.git(&["update-ref", "refs/remotes/origin/main", "HEAD"]);
    fixture.git(&["checkout", "-b", "topic"]);

    let package = fixture.root().join("packages/runtime");
    let noncanonical = fixture.root().join("packages/runtime/../runtime");
    for root in [&package, &noncanonical] {
        assert_eq!(
            comparison_counts_at(root, &["tests/guarded.rs"])
                .expect("nested package comparison must use its Git prefix"),
            (1, 1)
        );
    }
    assert_eq!(
        comparison_counts_at(&package, &["packages/runtime/tests/guarded.rs"])
            .expect("already-prefixed paths must retain their Git identity"),
        (1, 1)
    );
}

#[test]
fn nested_package_changes_keep_git_prefixed_paths_current() {
    let fixture = git_fixture();
    fixture.write("packages/runtime/tests/control.rs", "fn control() {}\n");
    fixture.commit_all("base nested control");
    fixture.git(&["branch", "-M", "main"]);
    fixture.git(&["update-ref", "refs/remotes/origin/main", "HEAD"]);
    fixture.git(&["checkout", "-b", "topic"]);
    fixture.write(
        "packages/runtime/tests/new_assertion.rs",
        "let skill = std::fs::read_to_string(\"plugins/codexy/skills/demo/SKILL.md\")?;\nassert!(skill.contains(\"heading\"));\n",
    );
    fixture.commit_all("topic adds nested assertion");

    assert_eq!(
        repository_violations_at(&fixture.root().join("packages/runtime"))
            .expect("nested package changes must resolve Git-prefixed paths"),
        ["tests/new_assertion.rs: line 2 receiver `skill`"]
    );
}

#[test]
fn missing_paths_and_unrelated_git_roots_fail_closed() {
    let fixture = git_fixture();
    fixture.write("tests/control.rs", "fn control() {}\n");
    fixture.write("unrelated/marker.rs", "fn marker() {}\n");
    fixture.commit_all("base roots");
    fixture.git(&["branch", "-M", "main"]);
    fixture.git(&["update-ref", "refs/remotes/origin/main", "HEAD"]);

    assert!(comparison_counts_at(fixture.root(), &["tests/missing.rs"]).is_err());
    assert!(comparison_counts_at(fixture.root(), &["../tests/control.rs"]).is_err());
    let absolute = fixture.root().join("tests/control.rs");
    assert!(comparison_counts_at(fixture.root(), &[absolute.to_str().expect("path is UTF-8")]).is_err());
    assert!(
        comparison_counts_at(&fixture.root().join("unrelated"), &["tests/control.rs"]).is_err()
    );
}
