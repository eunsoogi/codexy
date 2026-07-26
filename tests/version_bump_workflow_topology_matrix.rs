use super::version_bump_workflow_contract::validate_version_pr_publication;

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
    let post_mutation_identity = connected.replace(
        "observed_pr_args=(--observed-pr-json \"$state_dir/observed-pr.json\")",
        "observed_pr_args=(--observed-pr-json \"$state_dir/pr-state.base.json\")",
    );
    let cases = [
        ("valid connected topology", workflow("open-version-pr", "Open version bump pull request", false), connected.clone(), true),
        ("distinct valid function syntax", workflow("open-version-pr", "Open version bump pull request", false), distinct_valid, true),
        ("distinct valid unrelated step", workflow("open-version-pr", "Open version bump pull request", true), connected.clone(), true),
        ("empty publisher", workflow("open-version-pr", "Open version bump pull request", false), empty_publisher, false),
        ("disconnected mutation", workflow("open-version-pr", "Open version bump pull request", false), disconnected_mutation, false),
        ("extra disconnected mutation", workflow("open-version-pr", "Open version bump pull request", false), extra_disconnected, false),
        ("post-mutation state cannot authorize identity", workflow("open-version-pr", "Open version bump pull request", false), post_mutation_identity, false),
        ("wrong job", workflow("publish-version-pr", "Open version bump pull request", false), connected.clone(), false),
        ("wrong step", workflow("open-version-pr", "Publish something else", false), connected, false),
        ("wrong transaction order", workflow("open-version-pr", "Open version bump pull request", false), wrong_order, false),
    ];
    let mismatches = cases
        .into_iter()
        .filter_map(|(name, workflow, adapter, expected)| {
            (validate_version_pr_publication(&workflow, &adapter).is_ok() != expected)
                .then_some(name)
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
gh pr view "$pr_number" --repo "$GITHUB_REPOSITORY" \
  --json body,headRefName,labels,closingIssuesReferences \
  > "$state_dir/observed-pr.json"
observed_pr_args=(--observed-pr-json "$state_dir/observed-pr.json")
action=$(scripts/plan-version-pr-reconciliation \
  --has-changes true \
  --issue-json "$state_dir/issue.json" "${observed_pr_args[@]}")
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

fn workflow(job: &str, step: &str, unrelated_step: bool) -> String {
    let mut workflow = format!("jobs:\n  {job}:\n    steps:\n");
    if unrelated_step {
        workflow.push_str("      - name: Prepare\n        run: echo ready\n");
    }
    workflow.push_str(&format!(
        "      - name: {step}\n        run: scripts/reconcile-version-pr\n"
    ));
    workflow
}
