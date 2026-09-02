use std::fs;

use crate::support;

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
        match state {
            RemoteTag::ConcurrentWrong | RemoteTag::ConcurrentUnpeelable => {
                assert_create_reference_diagnostic(&output, "422", "HTTP/2.0 422 Unprocessable Entity");
            }
            RemoteTag::ApiAuth => {
                assert_create_reference_diagnostic(&output, "401", "HTTP/2.0 401 Unauthorized");
            }
            RemoteTag::ApiFailure => {
                assert_create_reference_diagnostic(&output, "500", "HTTP/2.0 500 Server Error");
            }
            _ => {}
        }
        if matches!(state, RemoteTag::ConcurrentWrong) {
            assert_eq!(fixture.remote_state()?, "wrong", "API did not set wrong remote ref");
            for (name, expected) in [("git", 7), ("jq", 3), ("gh", 2)] {
                assert_eq!(fixture.command_calls(name)?, expected, "{name} host fallthrough");
            }
        }
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
        if matches!(state, RemoteTag::ConcurrentExact) {
            assert_create_reference_diagnostic(
                &output,
                "422",
                "HTTP/2.0 422 Unprocessable Entity",
            );
        }
    }
    Ok(())
}

#[cfg(unix)]
#[test]
fn create_reference_diagnostic_and_auth_are_bounded_and_credential_safe()
-> Result<(), Box<dyn std::error::Error>> {
    let publisher = fs::read_to_string(
        codexy_runtime::paths::repository_root().join("scripts/publish-verified-release"),
    )?;
    let temp = tempfile::tempdir()?;
    let response = temp.path().join("response");
    fs::write(&response, format!(
        "HTTP/2.0 422 Unprocessable Entity\n{}\n\n{{\"token\":\"fixture-token\"}}\n{}",
        "header\n".repeat(25), "body\n".repeat(200)
    ))?;
    let probe = temp.path().join("probe.sh");
    fs::write(&probe, format!(
        "#!/bin/sh\n{}\ntag_create_diagnostic 422 \"$1\"\n",
        extract_function(&publisher, "tag_create_diagnostic")?
    ))?;
    let output = std::process::Command::new("sh")
        .arg(&probe)
        .arg(&response)
        .output()?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("HTTP/2.0 422 Unprocessable Entity body=body"));
    assert!(!stderr.contains("fixture-token"), "diagnostic leaked fixture credential");
    assert!(stderr.len() < 700, "diagnostic exceeded bound: {}", stderr.len());

    let bin = temp.path().join("bin"); fs::create_dir(&bin)?;
    let git = bin.join("git");
    fs::write(&git, "#!/bin/sh\nset -eu\ntest \"$1\" = ls-remote\ntest \"${GIT_CONFIG_COUNT:-}\" = 1\ntest \"${GIT_CONFIG_KEY_0:-}\" = http.extraheader\ntest \"${GIT_CONFIG_VALUE_0:-}\" = 'Authorization: Basic eC1hY2Nlc3MtdG9rZW46Zml4dHVyZS10b2tlbg=='\ncase \"${GIT_CONFIG_VALUE_0:-}\" in *fixture-token*) exit 1 ;; esac\nprintf '%s\\n' basic-auth-probe-ok\n")?;
    support::make_executable(&git)?;
    let probe = temp.path().join("auth-probe.sh");
    fs::write(&probe, format!(
        "#!/bin/sh\n{}\ntag_ref=refs/tags/v1.6.0\nauthenticated_remote_tag\n",
        extract_function(&publisher, "authenticated_remote_tag")?
    ))?;
    let host_path = std::env::var_os("PATH").ok_or("PATH")?;
    let path = std::env::join_paths(std::iter::once(bin.clone()).chain(std::env::split_paths(&host_path)))?;
    let output = std::process::Command::new("sh")
        .arg(&probe)
        .env("PATH", path)
        .env("GH_TOKEN", "fixture-token")
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "Basic auth probe failed: {stdout}");
    assert!(stdout.contains("basic-auth-probe-ok")
        && !stdout.contains("fixture-token")
        && !String::from_utf8_lossy(&output.stderr).contains("fixture-token"));
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

fn assert_create_reference_diagnostic(
    output: &std::process::Output,
    status: &str,
    response: &str,
) {
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(&format!("create-reference status={status} response=")),
        "missing create-reference status diagnostic: {stderr}"
    );
    assert!(
        stderr.contains(response),
        "missing bounded create-reference response: {stderr}"
    );
    assert!(!stderr.contains("fixture-token"), "diagnostic leaked fixture credential");
}

fn extract_function<'a>(
    source: &'a str,
    name: &str,
) -> Result<&'a str, Box<dyn std::error::Error>> {
    let start = source.find(&format!("{name}() {{")).ok_or("function start")?;
    let end = source[start..].find("\n}\nrelease_exists=false").ok_or("function end")? + 2;
    Ok(&source[start..start + end])
}
