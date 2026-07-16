use std::process::Command;

#[test]
fn new_contract_tests_cannot_add_unstructured_substring_assertions() {
    let output = Command::new("git")
        .args(["diff", "--unified=0", "origin/main", "--", "tests"])
        .output()
        .expect("git diff must run for the migration guard");
    assert!(
        output.status.success(),
        "migration guard could not inspect test changes"
    );
    let diff = String::from_utf8(output.stdout).expect("git diff is UTF-8");
    let violations: Vec<_> = diff
        .lines()
        .filter(|line| {
            line.starts_with('+')
                && !line.starts_with("+++")
                && line.contains("assert!")
                && line.contains(".contains(")
                && !line.contains("structured-contract: non-contract substring rationale:")
        })
        .collect();
    assert!(
        violations.is_empty(),
        "new raw substring assertions need a structured rule or a narrow non-contract rationale: {violations:?}"
    );
}
