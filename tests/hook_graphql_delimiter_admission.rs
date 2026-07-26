use super::hook_fail_closed_command_admission::{
    TestResult, assert_case, plugin_root, repository,
};

#[test]
fn graphql_delimiters_fail_closed_without_blocking_nested_controls() -> TestResult {
    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let foreign = repository(workspace.path(), "foreign", "https://github.com/openai/codex.git")?;

    for query in [
        "query { viewer(] { login } }",
        "query($x: [Int)] { viewer { login } }",
        "query {}",
        "query { # comment\n}",
    ] {
        assert_case(
            &root,
            &foreign,
            &format!("gh api graphql -f query='{query}'"),
            true,
            &[],
        )?;
    }
    assert_case(
        &root,
        &foreign,
        "gh api graphql -f query='query Query($x: [Int!] = [1, 2]) @cache { __typename viewer { repositories(first: 1, orderBy: {field: NAME, direction: ASC}, labels: [ONE, TWO], empty: []) { nodes { ...RepoFields } } } } fragment RepoFields on Repository { name }'",
        false,
        &[],
    )
}
