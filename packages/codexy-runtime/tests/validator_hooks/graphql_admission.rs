use super::admission_runtime::{TestResult, assert_case, plugin_root, repository};

#[test]
fn issue_735_graphql_queries_and_exact_mutations_are_classified_structurally() -> TestResult {
    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let owned = repository(workspace.path(), "owned", "git@github.com:eunsoogi/codexy.git")?;
    let bindings = "-f owner=eunsoogi -f name=codexy -f repository_id=REPO_owned -f issue_id=ISS_owned -f pull_request_id=PR_owned -f subject_id=ISS_owned -f labelable_id=ISS_owned -f assignable_id=ISS_owned -f duplicate_issue_id=ISS_duplicate -f label_ids='[\"LABEL\"]' -f assignee_ids='[\"USER\"]' -f user_ids='[\"USER\"]' -f user_logins='[\"eunsoogi\"]' -f client_mutation_id=CLIENT";
    for query in [
        "query { viewer { login } }",
        "query { search(query:\"mutation { mergePullRequest }\",type:ISSUE,first:1) { issueCount } }",
        "query { viewer { login } } # mutation { mergePullRequest }",
    ] {
        assert_case(&root, &owned, &format!("gh api graphql --jq '.data' -f query='{query}'"), false, &[])?;
    }
    for (case_id, query) in [
        ("P-ISS-01", "mutation { createIssue(input:{repositoryId:\"REPO_owned\",title:\"Valid issue\",clientMutationId:\"CLIENT\"}) { issue { number } } }"),
        ("P-ISS-02", "mutation { updateIssue(input:{issueId:\"ISS_owned\",title:\"Updated issue\"}) { issue { number } } }"),
        ("P-PR-01", "mutation { createPullRequest(input:{repositoryId:\"REPO_owned\",title:\"fix(hooks): create PR\",headRefName:\"topic\",baseRefName:\"main\"}) { pullRequest { number } } }"),
        ("P-PR-08", "mutation { markPullRequestReadyForReview(input:{pullRequestId:\"PR_owned\"}) { pullRequest { number } } }"),
    ] {
        assert_case(&root, &owned, &format!("gh api graphql {bindings} -f query='{query}'"), false, &[])
            .map_err(|error| format!("{case_id}: {error}"))?;
    }
    for (case_id, query) in [
        ("P-ISS-02-body-clear", "mutation { updateIssue(input:{issueId:\"ISS_owned\",body:null}) { issue { number } } }"),
        ("P-ISS-03", "mutation { closeIssue(input:{issueId:\"ISS_owned\",stateReason:DUPLICATE,duplicateIssueId:\"ISS_duplicate\"}) { issue { number } } }"),
        ("P-ISS-03-reopen", "mutation { reopenIssue(input:{issueId:\"ISS_owned\"}) { issue { number } } }"),
        ("P-ISS-04", "mutation { addComment(input:{subjectId:\"ISS_owned\",body:\"note\"}) { comment { id } } }"),
        ("P-ISS-05", "mutation { addLabelsToLabelable(input:{labelableId:\"ISS_owned\",labelIds:[\"LABEL\"]}) { labelable { id } } }"),
        ("P-ISS-06", "mutation { addAssigneesToAssignable(input:{assignableId:\"ISS_owned\",assigneeIds:[\"USER\"]}) { assignable { id } } }"),
        ("P-ISS-07", "mutation { updateIssue(input:{issueId:\"ISS_owned\",milestoneId:null}) { issue { number } } }"),
        ("P-PR-03", "mutation { closePullRequest(input:{pullRequestId:\"PR_owned\"}) { pullRequest { number } } }"),
        ("P-PR-05", "mutation { addPullRequestReview(input:{pullRequestId:\"PR_owned\",event:APPROVE,fileComments:null}) { pullRequestReview { id } } }"),
        ("P-PR-06", "mutation { requestReviews(input:{pullRequestId:\"PR_owned\",userIds:[\"USER\"],union:true}) { pullRequest { number } } }"),
        ("P-PR-06-login", "mutation { requestReviewsByLogin(input:{pullRequestId:\"PR_owned\",userLogins:[\"eunsoogi\"],union:false}) { pullRequest { number } } }"),
        ("P-PR-07", "mutation { convertPullRequestToDraft(input:{pullRequestId:\"PR_owned\"}) { pullRequest { number } } }"),
    ] {
        assert_case(&root, &owned, &format!("gh api graphql {bindings} -f query='{query}'"), false, &[])
            .map_err(|error| format!("{case_id}: {error}"))?;
    }
    let clear_bindings = bindings
        .replace("label_ids='[\"LABEL\"]'", "label_ids='[]'")
        .replace("assignee_ids='[\"USER\"]'", "assignee_ids='[]'");
    for (case_id, query) in [
        ("P-ISS-05-clear", "mutation { updateIssue(input:{issueId:\"ISS_owned\",labelIds:[]}) { issue { number } } }"),
        ("P-ISS-06-clear", "mutation { updateIssue(input:{issueId:\"ISS_owned\",assigneeIds:[]}) { issue { number } } }"),
    ] {
        assert_case(&root, &owned, &format!("gh api graphql {clear_bindings} -f query='{query}'"), false, &[])
            .map_err(|error| format!("{case_id}: {error}"))?;
    }
    for (case_id, query) in [
        ("N-12", "mutation { deleteProjectV2(input:{projectV2Id:\"fixture\"}) { clientMutationId } }"),
        ("N-12-extra", "mutation { createIssue(input:{repositoryId:\"REPO_owned\",title:\"Valid issue\",unexpected:\"no\"}) { issue { number } } }"),
        ("N-12-multi", "mutation { createIssue(input:{repositoryId:\"REPO_owned\",title:\"Valid issue\"}) { issue { number } } } mutation { reopenIssue(input:{issueId:\"ISS_owned\"}) { issue { number } } }"),
        ("N-12-alias", "mutation { alias:createIssue(input:{repositoryId:\"REPO_owned\",title:\"Valid issue\"}) { issue { number } } }"),
        ("N-13-foreign-node", "mutation { updateIssue(input:{issueId:\"ISS_foreign\",title:\"Updated issue\"}) { issue { number } } }"),
        ("N-17-review-without-body", "mutation { addPullRequestReview(input:{pullRequestId:\"PR_owned\",event:COMMENT}) { pullRequestReview { id } } }"),
    ] {
        assert_case(&root, &owned, &format!("gh api graphql {bindings} -f query='{query}'"), true, &[])
            .map_err(|error| format!("{case_id}: {error}"))?;
    }
    Ok(())
}

#[test]
fn graphql_delimiters_fail_closed_without_blocking_nested_controls() -> TestResult {
    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let foreign = repository(workspace.path(), "foreign", "https://github.com/openai/codex.git")?;
    for query in ["", "# comment only", "query { viewer(] { login } }", "query($x: [Int)] { viewer { login } }", "query {}", "query { # comment\n}", "query { ... }", "query { viewer {} }", "fragment { viewer }", "query() { viewer { login } }"] { assert_case(&root, &foreign, &format!("gh api graphql -f query='{query}'"), true, &[])?; }
    for query in ["{ __typename }", "query Query($x: [Int!] = [1, 2]) @cache { __typename viewer { repositories(first: 1, orderBy: {field: NAME, direction: ASC}, labels: [ONE, TWO], empty: []) { nodes { ...RepoFields ... on Repository { name } } } } } fragment RepoFields on Repository { name }"] { assert_case(&root, &foreign, &format!("gh api graphql -f query='{query}'"), false, &[])?; }
    Ok(())
}
