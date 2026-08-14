use super::*;

#[test]
fn launcher_binding_preserves_native_payload_arguments_across_the_posix_shell()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let script = temp.path().join("release-helper");
    let bridge = temp.path().join("native-bridge");
    let payload = temp.path().join("gh-payload");
    write_posix_fixture_command(
        &script,
        "#!/bin/sh\ngh repos/eunsoogi/codexy /d/workspace/asset\n",
    )?;
    write_posix_fixture_command(
        &bridge,
        "#!/bin/sh\ntest -z \"${MSYS_NO_PATHCONV:-}\" || exit 75\ntest \"${MSYS2_ARG_CONV_EXCL:-}\" = 'repos/*;eunsoogi/codexy' || exit 76\nexec sh \"$@\"\n",
    )?;
    write_posix_fixture_command(&payload, "#!/bin/sh\nprintf '%s|%s\\n' \"$1\" \"$2\"\n")?;
    bind_posix_fixture_shell_launchers(
        &script,
        &[(
            "gh",
            "CODEXY_FIXTURE_GH",
            "CODEXY_FIXTURE_GH_LAUNCHER",
            FixtureArgumentDomain::GitHubApi,
        )],
    )?;
    let output = FixtureCommand::new(&script)
        .env("CODEXY_FIXTURE_GH", &payload)
        .env_path("CODEXY_FIXTURE_GH_LAUNCHER", &bridge)
        .env("GITHUB_REPOSITORY", "eunsoogi/codexy")
        .output()?;
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        normalize_fixture_text(&String::from_utf8(output.stdout)?),
        "repos/eunsoogi/codexy|/d/workspace/asset\n"
    );
    Ok(())
}

#[test]
fn launcher_binding_marshals_native_mock_paths_and_logical_repository_values()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let script = temp.path().join("release-helper");
    let gh = temp.path().join("gh");
    let state = temp.path().join("native state.json");
    write_posix_fixture_command(
        &script,
        "#!/bin/sh\ngh release view v9.9.9 --repo \"$GITHUB_REPOSITORY\"\ngh api \"repos/$GITHUB_REPOSITORY/releases/tags/v9.9.9\" --dir \"$FIXTURE_STATE\"\n",
    )?;
    fs::write(
        &gh,
        "#!/usr/bin/env python3\nimport os,pathlib,sys\nrepo='eunsoogi/codexy'\nassert os.environ['MSYS2_ARG_CONV_EXCL'] == f'repos/*;{repo}'\nargs=sys.argv[1:]\nif args[:2] == ['release', 'view']:\n assert args == ['release', 'view', 'v9.9.9', '--repo', repo]\n print('release:' + args[-1])\nelif args[:1] == ['api']:\n assert args[:2] == ['api', f'repos/{repo}/releases/tags/v9.9.9']\n assert args[2] == '--dir'\n pathlib.Path(args[3]).write_bytes(b'native state\\r\\n')\n print('api:' + args[1])\nelse:\n raise AssertionError(args)\n",
    )?;
    crate::support::make_executable(&gh)?;
    bind_posix_fixture_shell_launchers(
        &script,
        &[(
            "gh",
            "CODEXY_FIXTURE_GH",
            "CODEXY_FIXTURE_GH_LAUNCHER",
            FixtureArgumentDomain::GitHubApi,
        )],
    )?;
    let output = FixtureCommand::new(&script)
        .env("CODEXY_FIXTURE_GH", &gh)
        .env_path(
            "CODEXY_FIXTURE_GH_LAUNCHER",
            fixture_script_interpreter_path(&gh)?,
        )
        .env_path("FIXTURE_STATE", &state)
        .env("GITHUB_REPOSITORY", "eunsoogi/codexy")
        .output()?;
    assert!(
        output.status.success(),
        "stdout: {} stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        normalize_fixture_text(&String::from_utf8(output.stdout)?),
        "release:eunsoogi/codexy\napi:repos/eunsoogi/codexy/releases/tags/v9.9.9\n"
    );
    assert_eq!(
        normalize_fixture_text(&fs::read_to_string(state)?),
        "native state\n"
    );
    Ok(())
}

#[test]
fn launcher_binding_leaves_posix_filesystem_operands_under_default_conversion()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let script = temp.path().join("release-helper");
    let bridge = temp.path().join("posix-bridge");
    let payload = temp.path().join("gh-payload");
    write_posix_fixture_command(&script, "#!/bin/sh\ngh /tmp/release-download\n")?;
    write_posix_fixture_command(
        &bridge,
        "#!/bin/sh\ntest -z \"${MSYS_NO_PATHCONV:-}\" || exit 75\ntest -z \"${MSYS2_ARG_CONV_EXCL:-}\" || exit 76\nexec sh \"$@\"\n",
    )?;
    write_posix_fixture_command(&payload, "#!/bin/sh\nprintf '%s\\n' \"$1\"\n")?;
    bind_posix_fixture_shell_launchers(
        &script,
        &[(
            "gh",
            "CODEXY_FIXTURE_GH",
            "CODEXY_FIXTURE_GH_LAUNCHER",
            FixtureArgumentDomain::Posix,
        )],
    )?;
    let output = FixtureCommand::new(&script)
        .env_path("CODEXY_FIXTURE_GH", &payload)
        .env_path("CODEXY_FIXTURE_GH_LAUNCHER", &bridge)
        .output()?;
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        normalize_fixture_text(&String::from_utf8(output.stdout)?),
        "/tmp/release-download\n"
    );
    Ok(())
}
