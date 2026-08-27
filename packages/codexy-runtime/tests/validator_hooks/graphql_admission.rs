use super::admission_runtime::{TestResult, assert_case, plugin_root, repository};

#[test]
fn issue_735_graphql_queries_and_exact_mutations_are_classified_structurally() -> TestResult {
    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let owned = repository(workspace.path(), "owned", "git@github.com:eunsoogi/codexy.git")?;
    for query in [
        "query { viewer { login } }",
        "query { search(query:\"mutation { mergePullRequest }\",type:ISSUE,first:1) { issueCount } }",
        "query { viewer { login } } # mutation { mergePullRequest }",
    ] {
        assert_case(&root, &owned, &format!("gh api graphql --jq '.data' -f query='{query}'"), false, &[])?;
    }
    for query in [
        "mutation { createIssue(input:{repositoryId:\"REPO_owned\",title:\"Valid issue\"}) { issue { number } } }",
        "mutation { updateIssue(input:{issueId:\"ISS_owned\",title:\"Updated issue\"}) { issue { number } } }",
        "mutation { createPullRequest(input:{repositoryId:\"REPO_owned\",title:\"fix(hooks): create PR\",headRefName:\"topic\",baseRefName:\"main\"}) { pullRequest { number } } }",
        "mutation { markPullRequestReadyForReview(input:{pullRequestId:\"PR_owned\"}) { pullRequest { number } } }",
    ] {
        assert_case(&root, &owned, &format!("gh api graphql -f owner=eunsoogi -f name=codexy -f query='{query}'"), false, &[])?;
    }
    for query in [
        "mutation { updateIssue(input:{issueId:\"ISS_owned\",body:null}) { issue { number } } }",
        "mutation { closeIssue(input:{issueId:\"ISS_owned\",stateReason:DUPLICATE,duplicateIssueId:\"ISS_duplicate\"}) { issue { number } } }",
        "mutation { reopenIssue(input:{issueId:\"ISS_owned\"}) { issue { number } } }",
        "mutation { addComment(input:{subjectId:\"ISS_owned\",body:\"note\"}) { comment { id } } }",
        "mutation { addLabelsToLabelable(input:{labelableId:\"ISS_owned\",labelIds:[\"LABEL\"]}) { labelable { id } } }",
        "mutation { removeLabelsFromLabelable(input:{labelableId:\"ISS_owned\",labelIds:[]}) { labelable { id } } }",
        "mutation { addAssigneesToAssignable(input:{assignableId:\"ISS_owned\",assigneeIds:[\"USER\"]}) { assignable { id } } }",
        "mutation { updateIssue(input:{issueId:\"ISS_owned\",milestoneId:null}) { issue { number } } }",
        "mutation { closePullRequest(input:{pullRequestId:\"PR_owned\"}) { pullRequest { number } } }",
        "mutation { addPullRequestReview(input:{pullRequestId:\"PR_owned\",event:APPROVE,fileComments:null}) { pullRequestReview { id } } }",
        "mutation { requestReviews(input:{pullRequestId:\"PR_owned\",userIds:[\"USER\"],union:true}) { pullRequest { number } } }",
        "mutation { requestReviewsByLogin(input:{pullRequestId:\"PR_owned\",userLogins:[\"eunsoogi\"],union:false}) { pullRequest { number } } }",
        "mutation { convertPullRequestToDraft(input:{pullRequestId:\"PR_owned\"}) { pullRequest { number } } }",
    ] {
        assert_case(&root, &owned, &format!("gh api graphql -f owner=eunsoogi -f name=codexy -f query='{query}'"), false, &[])?;
    }
    for query in [
        "mutation { deleteProjectV2(input:{projectV2Id:\"fixture\"}) { clientMutationId } }",
        "mutation { createIssue(input:{repositoryId:\"REPO_owned\",title:\"Valid issue\",unexpected:\"no\"}) { issue { number } } }",
        "mutation { createIssue(input:{repositoryId:\"REPO_owned\",title:\"Valid issue\"}) { issue { number } } } mutation { reopenIssue(input:{issueId:\"ISS_owned\"}) { issue { number } } }",
        "mutation { alias:createIssue(input:{repositoryId:\"REPO_owned\",title:\"Valid issue\"}) { issue { number } } }",
    ] {
        assert_case(&root, &owned, &format!("gh api graphql -f owner=eunsoogi -f name=codexy -f query='{query}'"), true, &[])?;
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
