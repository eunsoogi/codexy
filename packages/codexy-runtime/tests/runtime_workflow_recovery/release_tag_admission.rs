use std::fs;

#[path = "release_tag_admission/fixture.rs"]
mod fixture;

use fixture::{Fixture, RemoteTag};

#[test]
fn fixture_error_context_names_missing_fixture() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let path = temp.path().join("missing fixture.sh");
    fixture::assert_fixture_error_context(&path, temp.path(), false)
}

#[cfg(unix)]
#[test]
fn fixture_error_context_names_non_executable_fixture() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let path = temp.path().join("non-executable fixture.sh");
    fs::write(&path, "#!/bin/sh\nexit 0\n")?;
    fixture::assert_fixture_error_context(&path, temp.path(), true)
}

#[test]
fn remote_version_tag_admission_uses_authenticated_create_only_api()
-> Result<(), Box<dyn std::error::Error>> {
    for state in [
        RemoteTag::Wrong,
        RemoteTag::Unpeelable,
        RemoteTag::Changed,
        RemoteTag::ExactOutsideProtectedMain,
        RemoteTag::ExactLosesProtectedMainAfterSource,
        RemoteTag::AbsentAfterMainAdvance,
        RemoteTag::ConcurrentWrong,
        RemoteTag::ConcurrentUnpeelable,
        RemoteTag::ApiAuth,
        RemoteTag::ApiFailure,
    ] {
        let fixture = Fixture::new(state)?;
        let output = fixture.run()?;
        assert!(
            !output.status.success(),
            "unsafe {state:?} tag unexpectedly admitted"
        );
        assert_eq!(
            fixture.release_calls()?,
            0,
            "{state:?} reached release creation: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            fixture.git_push_calls()?,
            0,
            "{state:?} used unauthenticated git push"
        );
        assert_eq!(
            fixture.api_calls()?,
            state.create_api_calls(),
            "{state:?} API admission count"
        );
    }
    for state in [
        RemoteTag::Exact,
        RemoteTag::ExactAfterMainAdvance,
        RemoteTag::Absent,
        RemoteTag::ConcurrentExact,
    ] {
        let fixture = Fixture::new(state)?;
        let output = fixture.run()?;
        assert!(
            !output.status.success(),
            "fixture must stop at fake release boundary"
        );
        assert!(String::from_utf8_lossy(&output.stderr).contains("release-create sentinel"));
        assert_eq!(
            fixture.release_calls()?,
            1,
            "{state:?} tag did not admit release"
        );
        assert_eq!(
            fixture.git_push_calls()?,
            0,
            "{state:?} used unauthenticated git push"
        );
        assert_eq!(
            fixture.api_calls()?,
            state.create_api_calls(),
            "{state:?} API admission count"
        );
    }
    Ok(())
}

#[test]
fn concurrent_wrong_uses_only_fixture_commands_before_rejection()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = Fixture::new(RemoteTag::ConcurrentWrong)?;
    let output = fixture.run()?;
    assert!(
        !output.status.success(),
        "concurrent wrong tag unexpectedly admitted"
    );
    assert_eq!(fixture.api_calls()?, 1, "authenticated API was not called");
    assert_eq!(
        fixture.remote_state()?,
        "wrong",
        "API did not set wrong remote ref"
    );
    assert_eq!(
        fixture.release_calls()?,
        0,
        "wrong tag reached release creation"
    );
    assert_eq!(fixture.command_calls("git")?, 7, "host git fallthrough");
    assert_eq!(fixture.command_calls("jq")?, 3, "host jq fallthrough");
    assert_eq!(fixture.command_calls("gh")?, 2, "host gh fallthrough");
    Ok(())
}

#[test]
fn fixture_discards_every_inherited_git_and_github_state() -> Result<(), Box<dyn std::error::Error>>
{
    assert_inherited_state_discarded(&[
        ("GIT_DIR", "host-git-dir"),
        ("GIT_WORK_TREE", "host-work-tree"),
        ("GIT_INDEX_FILE", "host-index"),
        ("GIT_COMMON_DIR", "host-common"),
        ("GH_CONFIG_DIR", "host-gh-config"),
        ("GH_HOST", "host-gh"),
        ("GH_ENTERPRISE_TOKEN", "host-enterprise-token"),
        ("GH_TOKEN", "host-gh-token"),
        ("GITHUB_TOKEN", "host-token"),
    ])
}

fn assert_inherited_state_discarded(
    poison: &[(&str, &str)],
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = Fixture::new(RemoteTag::ConcurrentWrong)?;
    let output = fixture.run_with_inherited_state(poison)?;
    assert!(
        !output.status.success(),
        "inherited state admitted concurrent wrong tag"
    );
    assert_eq!(
        fixture.api_calls()?,
        1,
        "inherited state blocked authenticated API"
    );
    assert_eq!(
        fixture.release_calls()?,
        0,
        "inherited state reached release"
    );
    assert_eq!(
        fixture.git_push_calls()?,
        0,
        "inherited state used git push"
    );
    assert_eq!(
        fixture.command_calls("git")?,
        7,
        "inherited Git state leaked"
    );
    assert_eq!(
        fixture.command_calls("jq")?,
        3,
        "inherited state leaked into jq"
    );
    assert_eq!(fixture.command_calls("gh")?, 2, "inherited state leaked");
    Ok(())
}
