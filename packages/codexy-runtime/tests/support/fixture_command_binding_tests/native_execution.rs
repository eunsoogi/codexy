use super::*;
use crate::support::fixture_github_argv_adapter_path;

#[test]
fn launcher_binding_projects_posix_payload_before_native_execution()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let bin = temp.path().join("bin");
    fs::create_dir(&bin)?;
    let script = temp.path().join("release-helper");
    let payload = temp.path().join("gh-payload");
    let projection = temp.path().join("payload-projection");
    write_posix_fixture_command(&script, "#!/bin/sh\ngh release view\n")?;
    write_posix_fixture_command(
        &payload,
        "#!/bin/sh\ntest \"$#\" = 2 || exit 62\nprintf 'gh:%s %s\\n' \"$1\" \"$2\"\n",
    )?;
    write_posix_fixture_command(
        &bin.join("cygpath"),
        "#!/bin/sh\ntest \"$1\" = -u && test \"$2\" = -- || exit 63\nprintf '%s\\n' \"$3\" > \"$FIXTURE_PAYLOAD_PROJECTION\"\nprintf '%s\\n' \"$3\"\n",
    )?;
    bind_posix_fixture_shell_launchers(
        &script,
        &[(
            "gh",
            "CODEXY_FIXTURE_GH",
            "CODEXY_FIXTURE_GH_LAUNCHER",
            FixtureArgumentDomain::GitHubApi {
                adapter_launcher_environment: "CODEXY_FIXTURE_GH_ADAPTER_LAUNCHER",
            },
        )],
    )?;
    let output = FixtureCommand::new(&script)
        .env_native_path("CODEXY_FIXTURE_GH", &payload)
        .env_native_path(
            "CODEXY_FIXTURE_GH_LAUNCHER",
            fixture_script_interpreter_path(&payload)?,
        )
        .env_path(
            "CODEXY_FIXTURE_GH_ADAPTER_LAUNCHER",
            fixture_script_interpreter_path(&fixture_github_argv_adapter_path(&script))?,
        )
        .env_path("FIXTURE_PAYLOAD_PROJECTION", &projection)
        .env_path("PATH", &bin)
        .env("CODEXY_FIXTURE_FORCE_NATIVE_WINDOWS", "1")
        .env("GITHUB_REPOSITORY", "eunsoogi/codexy")
        .output()?;
    assert!(
        output.status.success(),
        "stdout: {} stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8(output.stdout)?, "gh:release view\n");
    assert!(
        projection.is_file(),
        "the native adapter must project a POSIX payload before launch"
    );
    assert_eq!(
        fs::read_to_string(projection)?,
        format!("{}\n", payload.display())
    );
    Ok(())
}

#[test]
fn launcher_binding_moves_spaced_native_launch_paths_through_transport()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let script = temp.path().join("release-helper");
    let payload = temp.path().join("native payload");
    let adapter_launcher = temp.path().join("adapter launcher with spaces");
    let empty_path = temp.path().join("empty-path");
    fs::create_dir(&empty_path)?;
    write_posix_fixture_command(&script, "#!/bin/sh\ngh release view\n")?;
    write_posix_fixture_command(
        &payload,
        "#!/bin/sh\ntest \"$#\" = 2 || exit 62\nprintf 'gh:%s %s\\n' \"$1\" \"$2\"\n",
    )?;
    bind_posix_fixture_shell_launchers(
        &script,
        &[(
            "gh",
            "CODEXY_FIXTURE_GH",
            "CODEXY_FIXTURE_GH_LAUNCHER",
            FixtureArgumentDomain::GitHubApi {
                adapter_launcher_environment: "CODEXY_FIXTURE_GH_ADAPTER_LAUNCHER",
            },
        )],
    )?;
    write_posix_fixture_command(
        &adapter_launcher,
        "#!/bin/sh\ntest \"$#\" = 1 || exit 61\nexec \"$FIXTURE_ADAPTER_PYTHON\" \"$@\"\n",
    )?;
    let output = FixtureCommand::new(&script)
        .env_native_path("CODEXY_FIXTURE_GH", &payload)
        .env_native_path(
            "CODEXY_FIXTURE_GH_LAUNCHER",
            fixture_script_interpreter_path(&payload)?,
        )
        .env_path("CODEXY_FIXTURE_GH_ADAPTER_LAUNCHER", &adapter_launcher)
        .env_path(
            "FIXTURE_ADAPTER_PYTHON",
            fixture_script_interpreter_path(&fixture_github_argv_adapter_path(&script))?,
        )
        .env_path("PATH", &empty_path)
        .env("GITHUB_REPOSITORY", "eunsoogi/codexy")
        .output()?;
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8(output.stdout)?, "gh:release view\n");
    Ok(())
}
