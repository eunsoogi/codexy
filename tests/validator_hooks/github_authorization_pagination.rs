use crate::support::{FixtureCommand as Command, make_executable};
use serde_json::{Value, json};

use super::admission_runtime::{TestResult, plugin_root, repository};

const HEAD: &str = "32b03a210b3defb2d29dd352283ea2488e60d893";

#[cfg(unix)]
#[test]
fn canonical_wrapper_accepts_authorization_on_a_later_comment_page() -> TestResult {
    let first_page = first_page();
    let pages = pages(first_page.clone(), vec![authorization("IC_later")]);
    let (output, merged) = wrapper(&state(first_page), &pages, false)?;
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert!(merged, "later-page authorization did not reach the admitted merge");
    Ok(())
}

#[cfg(unix)]
#[test]
fn canonical_wrapper_rejects_replay_on_a_later_comment_page() -> TestResult {
    let mut first_page = first_page();
    first_page[0] = authorization("IC_current");
    let pages = pages(first_page.clone(), vec![authorization("IC_replay")]);
    let (output, merged) = wrapper(&state(first_page), &pages, false)?;
    assert!(!output.status.success(), "later-page replay was accepted");
    assert!(!merged, "later-page replay reached merge");
    Ok(())
}

#[cfg(unix)]
#[test]
fn canonical_wrapper_rejects_paginated_query_failure() -> TestResult {
    let first = authorization("IC_current");
    let (output, merged) = wrapper(&state(vec![first.clone()]), &pages(vec![first], vec![]), true)?;
    assert!(!output.status.success(), "pagination failure was accepted");
    assert!(!merged, "pagination failure reached merge");
    Ok(())
}

#[cfg(unix)]
fn wrapper(first: &str, pages: &str, fail_paginated: bool) -> TestResult<(std::process::Output, bool)> {
    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let owned = repository(workspace.path(), "owned", "git@github.com:eunsoogi/codexy.git")?;
    let message = owned.join("message.txt");
    let body = owned.join("body.txt");
    let first_file = workspace.path().join("first.json");
    let pages_file = workspace.path().join("pages.json");
    let record = workspace.path().join("merge.txt");
    let bin = workspace.path().join("bin");
    std::fs::write(&message, "fix(workflow): require intent (#128)\n\nFixes #503\n")?;
    std::fs::write(&body, "Fixes #503\n")?;
    std::fs::write(&first_file, first)?;
    std::fs::write(&pages_file, pages)?;
    std::fs::create_dir(&bin)?;
    let gh = bin.join("gh");
    std::fs::write(&gh, "#!/bin/sh\nif [ \"$1\" = api ]; then\n  case \" $* \" in\n    *\" --paginate \"*) [ \"${CODEXY_GH_FAIL_PAGINATION:-}\" != 1 ] && cat \"$CODEXY_GH_PAGES\" ;;\n    *) cat \"$CODEXY_GH_FIRST\" ;;\n  esac\nelse\n  printf merge > \"$CODEXY_GH_RECORD\"\nfi\n")?;
    make_executable(&gh)?;
    let output = Command::new(root.join("hooks/codexy-authorized-squash-merge.sh"))
        .current_dir(&owned)
        .env("PATH", format!("{}:{}", bin.display(), std::env::var("PATH")?))
        .env("CODEXY_GH_FIRST", first_file)
        .env("CODEXY_GH_PAGES", pages_file)
        .env("CODEXY_GH_RECORD", &record)
        .env("CODEXY_GH_FAIL_PAGINATION", if fail_paginated { "1" } else { "0" })
        .args(["--expected-pr", "128", "--expected-issue", "503", "--merge-message-file"])
        .arg(message)
        .args(["--repo", "eunsoogi/codexy", "--match-head-commit", HEAD, "--subject", "fix(workflow): require intent (#128)", "--body-file"])
        .arg(body)
        .output()?;
    Ok((output, record.exists()))
}

#[cfg(unix)]
fn pages(first: Vec<Value>, second: Vec<Value>) -> String {
    serde_json::to_string(&vec![response(first, true), response(second, false)]).unwrap()
}

#[cfg(unix)]
fn state(comments: Vec<Value>) -> String {
    serde_json::to_string(&json!({"repository":"eunsoogi/codexy","number":128,"baseRefName":"main","headRefOid":HEAD,"comments":comments})).unwrap()
}

#[cfg(unix)]
fn first_page() -> Vec<Value> {
    (0..100).map(|number| ordinary_comment(&format!("IC_{number}"))).collect()
}

#[cfg(unix)]
fn response(comments: Vec<Value>, has_next: bool) -> Value {
    let cursor = has_next.then_some("cursor-1");
    json!({"data":{"repository":{"nameWithOwner":"eunsoogi/codexy","pullRequest":{"number":128,"baseRefName":"main","headRefOid":HEAD,"comments":{"nodes":comments,"pageInfo":{"hasNextPage":has_next,"endCursor":cursor}}}}}})
}

#[cfg(unix)]
fn ordinary_comment(id: &str) -> Value {
    json!({"id":id,"url":format!("https://github.com/eunsoogi/codexy/pull/128#issuecomment-{id}"),"body":"waiting","author":{"login":"member","association":"MEMBER"}})
}

#[cfg(unix)]
fn authorization(id: &str) -> Value {
    json!({"id":id,"url":format!("https://github.com/eunsoogi/codexy/pull/128#issuecomment-{id}"),"body":format!("AUTHORIZE REPOSITORY SQUASH CONTRACT: PR #128 BASE main HEAD {HEAD}"),"author":{"login":"maintainer","association":"MEMBER"}})
}
