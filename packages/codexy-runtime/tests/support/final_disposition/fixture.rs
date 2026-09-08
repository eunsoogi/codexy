use std::path::Path;

use serde_json::{Value, json};

use super::disposition_fixture;
use crate::support::{FixtureCommand, TestResult};

pub(crate) use disposition_fixture::GhFixture;

pub(crate) fn locator(issue: u64, pull_request: u64) -> Value {
    json!({
        "repository": "eunsoogi/codexy",
        "owningIssue": issue,
        "pullRequest": pull_request,
        "maintainerComment": disposition_fixture::FINAL_AUTHORITY_COMMENT
    })
}

pub(crate) fn write(
    root: &Path,
    issue: u64,
    pull_request: u64,
    base: &str,
    source_head: &str,
    current_head: &str,
    finding_id: &str,
) -> TestResult<GhFixture> {
    let sources = disposition_fixture::ci_sources(pull_request, base, current_head);
    let maintainer = disposition_fixture::final_authority_response(
        pull_request,
        issue,
        base,
        source_head,
        current_head,
        finding_id,
    );
    disposition_fixture::write_gh_fixture(root, &sources, &maintainer)
}

pub(crate) fn configure(command: &mut FixtureCommand, fixture: &GhFixture) {
    command
        .env_path_list("PATH", fixture.path.clone())
        .env_path("CODEXY_TEST_CI_RESPONSE", &fixture.ci)
        .env_path("CODEXY_TEST_REQUIRED_STATUS_RESPONSE", &fixture.required)
        .env_path("CODEXY_TEST_EXPECTED_CHECKS_RESPONSE", &fixture.expected)
        .env_path("CODEXY_TEST_CHECK_SUITES_RESPONSE", &fixture.suites)
        .env_path("CODEXY_TEST_MAINTAINER_RESPONSE", &fixture.maintainer);
}
