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
        "#!/bin/sh\ntest \"${MSYS_NO_PATHCONV:-}\" = 1 || exit 75\nexec sh \"$@\"\n",
    )?;
    write_posix_fixture_command(&payload, "#!/bin/sh\nprintf '%s|%s\\n' \"$1\" \"$2\"\n")?;
    bind_posix_fixture_shell_launchers(
        &script,
        &[("gh", "CODEXY_FIXTURE_GH", "CODEXY_FIXTURE_GH_LAUNCHER")],
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
        "repos/eunsoogi/codexy|/d/workspace/asset\n"
    );
    Ok(())
}
