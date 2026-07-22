use super::version_bump_workflow_model::validate_version_pr_publication;

#[test]
fn publication_topology_equivalence_matrix() {
    let connected = connected_run("publish_version_pr_metadata() {");
    let distinct_valid = connected_run("publish_version_pr_metadata () {");
    let empty_publisher = disconnected_run("  :", true);
    let disconnected_mutation = disconnected_run("  render_version_pr_metadata \"$phase\"", true);
    let wrong_order = connected.replace(
        "refresh_version_pr_snapshot\npublish_version_pr_metadata \"$publication_phase\"",
        "publish_version_pr_metadata \"$publication_phase\"\nrefresh_version_pr_snapshot",
    );
    let extra_disconnected = format!(
        "{connected}gh api --method PUT \\\n  \"repos/$GITHUB_REPOSITORY/issues/$pr_number/labels\"\n"
    );
    let cases = [
        ("valid connected topology", fixture("open-version-pr", "Open version bump pull request", &connected, false), true),
        ("distinct valid function syntax", fixture("open-version-pr", "Open version bump pull request", &distinct_valid, false), true),
        ("distinct valid unrelated step", fixture("open-version-pr", "Open version bump pull request", &connected, true), true),
        ("empty publisher", fixture("open-version-pr", "Open version bump pull request", &empty_publisher, false), false),
        ("disconnected mutation", fixture("open-version-pr", "Open version bump pull request", &disconnected_mutation, false), false),
        ("extra disconnected mutation", fixture("open-version-pr", "Open version bump pull request", &extra_disconnected, false), false),
        ("wrong job", fixture("publish-version-pr", "Open version bump pull request", &connected, false), false),
        ("wrong step", fixture("open-version-pr", "Publish something else", &connected, false), false),
        ("wrong transaction order", fixture("open-version-pr", "Open version bump pull request", &wrong_order, false), false),
    ];
    let mismatches = cases
        .into_iter()
        .filter_map(|(name, workflow, expected)| {
            (validate_version_pr_publication(&workflow).is_ok() != expected).then_some(name)
        })
        .collect::<Vec<_>>();
    assert!(mismatches.is_empty(), "topology mismatches: {mismatches:?}");
}

fn connected_run(header: &str) -> String {
    format!(
        r#"{header}
  phase=$1
  render_version_pr_metadata "$phase"
  title=$(sed -n '1p' "$state_dir/metadata/title.txt")
  gh api --method PUT \
    "repos/$GITHUB_REPOSITORY/issues/$pr_number/labels" \
    --input "$state_dir/metadata/labels.json"
  gh pr edit "$pr_number" \
    --title "$title" \
    --body-file "$state_dir/metadata/body.md"
}}
{}"#,
        valid_transaction()
    )
}

fn disconnected_run(body: &str, mutations_outside: bool) -> String {
    let mutations = if mutations_outside {
        r#"gh api --method PUT \
  "repos/$GITHUB_REPOSITORY/issues/$pr_number/labels" \
  --input "$state_dir/metadata/labels.json"
gh pr edit "$pr_number" \
  --title "$title" \
  --body-file "$state_dir/metadata/body.md"
"#
    } else {
        ""
    };
    format!("publish_version_pr_metadata() {{\n{body}\n}}\n{mutations}{}", valid_transaction())
}

fn valid_transaction() -> &'static str {
    r#"publication_phase=$(scripts/plan-version-pr-reconciliation \
  --merge-message-checked false)
refresh_version_pr_snapshot
render_version_pr_metadata "$publication_phase"
refresh_version_pr_snapshot
publish_version_pr_metadata "$publication_phase"
scripts/build-version-pr-state \
  --output "$state_dir/pr-state.json"
plugins/codexy/hooks/codexy-pr-label-check.sh \
  --pr-state-file "$state_dir/pr-state.json"
scripts/validate-plugin-config --check-completion-handoff \
  --pr-state-file "$state_dir/pr-state.json"
plugins/codexy/hooks/codexy-merge-message-check.sh \
  --merge-message-file "$state_dir/merge-message.txt"
publication_phase=$(scripts/plan-version-pr-reconciliation \
  --merge-message-checked true)
publish_version_pr_metadata "$publication_phase"
"#
}

fn fixture(job: &str, step: &str, run: &str, unrelated_step: bool) -> String {
    let mut workflow = format!("jobs:\n  {job}:\n    steps:\n");
    if unrelated_step {
        workflow.push_str("      - name: Prepare\n        run: echo ready\n");
    }
    workflow.push_str(&format!("      - name: {step}\n        run: |\n"));
    for line in run.lines() {
        workflow.push_str("          ");
        workflow.push_str(line);
        workflow.push('\n');
    }
    workflow
}
