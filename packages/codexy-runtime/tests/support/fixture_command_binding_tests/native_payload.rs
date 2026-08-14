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
        "#!/bin/sh\ntest -z \"${MSYS_NO_PATHCONV:-}\" || exit 75\ntest \"${MSYS2_ARG_CONV_EXCL:-}\" = 'repos/*' || exit 76\nexec sh \"$@\"\n",
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
