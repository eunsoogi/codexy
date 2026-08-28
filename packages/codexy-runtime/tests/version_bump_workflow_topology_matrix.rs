use std::fs;

use super::version_bump_workflow_contract::validate_version_pr_publication;

#[test]
fn publication_topology_uses_the_existing_reconciliation_command() {
    let root = codexy_runtime::paths::repository_root();
    let workflow = fs::read_to_string(root.join(".github/workflows/plugin-version-bump.yml"))
        .expect("version bump workflow");
    let adapter = fs::read_to_string(root.join("scripts/reconcile-version-pr"))
        .expect("version reconciliation command");
    validate_version_pr_publication(&workflow, &adapter)
        .expect("workflow must keep metadata mutation inside the reconciliation command");
}
