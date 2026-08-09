use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicUsize, Ordering},
};

use super::structured_contract_guard::{comparison_counts_at, repository_violations_at};

static FIXTURE_SEQUENCE: AtomicUsize = AtomicUsize::new(0);

pub(super) struct GitFixture(PathBuf);

impl GitFixture {
    pub(super) fn root(&self) -> &Path {
        &self.0
    }

    pub(super) fn write(&self, path: &str, contents: &str) {
        let path = self.root().join(path);
        fs::create_dir_all(path.parent().expect("fixture path has a parent"))
            .expect("fixture directory must be created");
        fs::write(path, contents).expect("fixture source must be written");
    }

    pub(super) fn git(&self, arguments: &[&str]) {
        let output = Command::new("git")
            .args(arguments)
            .current_dir(self.root())
            .output()
            .expect("fixture git command must run");
        assert!(
            output.status.success(),
            "git {arguments:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    pub(super) fn commit_all(&self, message: &str) {
        self.git(&["add", "."]);
        self.git(&["commit", "-m", message]);
    }
}

impl Drop for GitFixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

pub(super) fn git_fixture() -> GitFixture {
    let suffix = FIXTURE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "codexy-structured-contract-{}-{suffix}",
        std::process::id()
    ));
    let fixture = GitFixture(root);
    fs::create_dir_all(fixture.root()).expect("fixture root must be created");
    fixture.git(&["init"]);
    fixture.git(&["config", "user.email", "guard@example.test"]);
    fixture.git(&["config", "user.name", "Structured Contract Guard"]);
    fixture
}

#[test]
fn ignores_assertions_changed_only_after_the_branch_merge_base() {
    let fixture = git_fixture();
    fixture.write(
        "tests/behind_main.rs",
        "let skill = std::fs::read_to_string(\"plugins/codexy/skills/demo/SKILL.md\")?;\nassert!(skill.contains(\"heading\"));\n",
    );
    fixture.commit_all("base assertion");
    fixture.git(&["branch", "-M", "main"]);
    fixture.git(&["checkout", "-b", "topic"]);
    fixture.git(&["checkout", "main"]);
    fixture.write(
        "tests/behind_main.rs",
        "let _skill = std::fs::read_to_string(\"plugins/codexy/skills/demo/SKILL.md\")?;\n",
    );
    fixture.commit_all("main removes assertion");
    fixture.git(&["update-ref", "refs/remotes/origin/main", "HEAD"]);
    fixture.git(&["checkout", "topic"]);

    assert_eq!(
        comparison_counts_at(fixture.root(), &["tests/behind_main.rs"])
            .expect("comparison must use the topic merge base"),
        (1, 1)
    );
    assert!(
        repository_violations_at(fixture.root())
            .expect("guard must inspect the topic merge base")
            .is_empty()
    );
}

#[test]
fn reports_a_governed_assertion_added_after_the_merge_base() {
    let fixture = git_fixture();
    fixture.write("tests/control.rs", "fn control() {}\n");
    fixture.commit_all("base control");
    fixture.git(&["branch", "-M", "main"]);
    fixture.git(&["update-ref", "refs/remotes/origin/main", "HEAD"]);
    fixture.git(&["checkout", "-b", "topic"]);
    fixture.write(
        "tests/new_assertion.rs",
        "let skill = std::fs::read_to_string(\"plugins/codexy/skills/demo/SKILL.md\")?;\nassert!(skill.contains(\"heading\"));\n",
    );
    fixture.commit_all("topic adds assertion");

    assert_eq!(
        repository_violations_at(fixture.root()).expect("guard must inspect added files"),
        ["tests/new_assertion.rs: line 2 receiver `skill`"]
    );
}

#[test]
fn reports_a_substring_assertion_after_an_assert_prefixed_helper_call() {
    let fixture = git_fixture();
    fixture.write("tests/control.rs", "fn control() {}\n");
    fixture.commit_all("base control");
    fixture.git(&["branch", "-M", "main"]);
    fixture.git(&["update-ref", "refs/remotes/origin/main", "HEAD"]);
    fixture.git(&["checkout", "-b", "topic"]);
    fixture.write(
        "tests/parsed_value.rs",
        concat!(
            "assert_search_metadata(&first, 1)?;\n",
            "assert!(first[\"matches\"][0].as_str()",
            ".is_some_and(|line| line.contains(\"ENTRY\")));\n",
        ),
    );
    fixture.commit_all("topic adds parsed-value substring assertion");

    assert_eq!(
        repository_violations_at(fixture.root())
            .expect("guard must reject the parsed-value substring assertion"),
        ["tests/parsed_value.rs: line 2 receiver `first`"]
    );
}

#[test]
fn rejects_a_same_line_assertion_body_change_after_the_merge_base() {
    let fixture = git_fixture();
    fixture.write(
        "tests/modified_assertion.rs",
        "let skill = std::fs::read_to_string(\"plugins/codexy/skills/demo/SKILL.md\")?;\nassert!(skill.contains(\"required policy\"));\n",
    );
    fixture.commit_all("base assertion");
    fixture.git(&["branch", "-M", "main"]);
    fixture.git(&["update-ref", "refs/remotes/origin/main", "HEAD"]);
    fixture.git(&["checkout", "-b", "topic"]);
    fixture.write(
        "tests/modified_assertion.rs",
        "let skill = std::fs::read_to_string(\"plugins/codexy/skills/demo/SKILL.md\")?;\nassert!(skill.contains(\"requiredpolicy\"));\n",
    );
    fixture.commit_all("topic changes assertion body");

    assert_eq!(
        repository_violations_at(fixture.root()).expect("guard must reject changed assertions"),
        ["tests/modified_assertion.rs: line 2 receiver `skill`"]
    );
}

#[test]
fn allows_an_unchanged_legacy_assertion_after_the_merge_base() {
    let fixture = git_fixture();
    fixture.write(
        "tests/legacy_assertion.rs",
        "let skill = std::fs::read_to_string(\"plugins/codexy/skills/demo/SKILL.md\")?;\nassert!(skill.contains(\"heading\"));\nlet marker = \"base\";\n",
    );
    fixture.commit_all("base assertion");
    fixture.git(&["branch", "-M", "main"]);
    fixture.git(&["update-ref", "refs/remotes/origin/main", "HEAD"]);
    fixture.git(&["checkout", "-b", "topic"]);
    fixture.write(
        "tests/legacy_assertion.rs",
        "let skill = std::fs::read_to_string(\"plugins/codexy/skills/demo/SKILL.md\")?;\nassert!(skill.contains(\"heading\"));\nlet marker = \"topic\";\n",
    );
    fixture.commit_all("topic changes a non-assertion line");

    assert!(
        repository_violations_at(fixture.root())
            .expect("guard must allow unchanged assertions")
            .is_empty()
    );
}

#[test]
fn ignores_a_deleted_governed_file_after_the_merge_base() {
    let fixture = git_fixture();
    fixture.write(
        "tests/deleted_assertion.rs",
        "let skill = std::fs::read_to_string(\"plugins/codexy/skills/demo/SKILL.md\")?;\nassert!(skill.contains(\"heading\"));\n",
    );
    fixture.commit_all("base assertion");
    fixture.git(&["branch", "-M", "main"]);
    fixture.git(&["update-ref", "refs/remotes/origin/main", "HEAD"]);
    fixture.git(&["checkout", "-b", "topic"]);
    fs::remove_file(fixture.root().join("tests/deleted_assertion.rs"))
        .expect("fixture source must be deleted");
    fixture.commit_all("topic deletes assertion");

    assert!(
        repository_violations_at(fixture.root())
            .expect("guard must ignore deleted files")
            .is_empty()
    );
}
