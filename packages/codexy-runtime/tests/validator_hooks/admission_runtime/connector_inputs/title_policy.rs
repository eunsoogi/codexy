use super::super::{TestResult, plugin_root};
use super::helpers::assert_connector_case;
use serde_json::json;

#[test]
fn connector_title_admission_shares_issue_pr_and_metadata_boundaries() -> TestResult {
    let root = plugin_root();
    for (case_id, tool, input) in [
        (
            "P-ISS-category-prose",
            "github_create_issue",
            json!({"repository_full_name":"eunsoogi/codexy","title":"CI fails when cache restore times out"}),
        ),
        (
            "P-PR-scoped-title",
            "github_create_pull_request",
            json!({"repository_full_name":"eunsoogi/codexy","title":"feat(task): desc","head_branch":"topic","base_branch":"main"}),
        ),
        (
            "P-PR-normalized-create",
            "github.create_pull_request",
            json!({"repository_full_name":"eunsoogi/codexy","title":"fix(hooks): create through normalized connector","head_branch":"topic","base_branch":"main"}),
        ),
        (
            "P-PR-normalized-update-title-only",
            "github.update_pull_request",
            json!({"repository_full_name":"eunsoogi/codexy","pr_number":17,"title":"fix(hooks): update through normalized connector"}),
        ),
        (
            "P-PR-normalized-update-title-body",
            "github.update_pull_request",
            json!({"repository_full_name":"eunsoogi/codexy","pr_number":17,"title":"fix(hooks): update title and body together","body":"note"}),
        ),
        (
            "P-ISS-metadata-only",
            "github_update_issue",
            json!({"repository_full_name":"eunsoogi/codexy","issue_number":17,"title":null,"body":"note"}),
        ),
        (
            "P-PR-metadata-only",
            "github_update_pull_request",
            json!({"repository_full_name":"eunsoogi/codexy","pr_number":17,"title":null,"body":"note"}),
        ),
    ] {
        assert_connector_case(&root, case_id, tool, input, false)?;
    }
    for (case_id, tool, input) in [
        (
            "N-ISS-spaced-category",
            "github_create_issue",
            json!({"repository_full_name":"eunsoogi/codexy","title":"CI : reduce build time"}),
        ),
        (
            "N-ISS-bare-category",
            "github_create_issue",
            json!({"repository_full_name":"eunsoogi/codexy","title":"Fix"}),
        ),
        (
            "N-PR-scope-less",
            "github_create_pull_request",
            json!({"repository_full_name":"eunsoogi/codexy","title":"feat: desc","head_branch":"topic","base_branch":"main"}),
        ),
        (
            "N-PR-number-decoration",
            "github_update_pull_request",
            json!({"repository_full_name":"eunsoogi/codexy","pr_number":17,"title":"feat(task): desc (#900)"}),
        ),
        (
            "N-PR-normalized-update-title-only-thread-title",
            "github.update_pull_request",
            json!({"repository_full_name":"eunsoogi/codexy","pr_number":953,"title":"#951 · PR #953 · Windows 원인 진단"}),
        ),
        (
            "N-PR-normalized-update-title-body-thread-title",
            "github.update_pull_request",
            json!({"repository_full_name":"eunsoogi/codexy","pr_number":953,"title":"#951 · PR #953 · Windows 원인 진단","body":"diagnostic body"}),
        ),
    ] {
        assert_connector_case(&root, case_id, tool, input, true)?;
    }
    Ok(())
}
