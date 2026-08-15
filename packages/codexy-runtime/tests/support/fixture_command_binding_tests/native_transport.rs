use super::*;

#[test]
fn launcher_binding_keeps_overlapping_github_invocations_isolated()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let script = temp.path().join("release-helper");
    let payload = temp.path().join("gh-payload");
    let adapter_launcher = temp.path().join("adapter-launcher");
    let counter = temp.path().join("adapter-counter");
    write_posix_fixture_command(
        &script,
        "#!/bin/sh\ngh release view first &\ngh release view second &\nwait\n",
    )?;
    fs::write(
        &payload,
        "#!/usr/bin/env python3\nimport sys\nvalue = sys.argv[-1]\nif value == 'first':\n import time\n time.sleep(0.1)\nelif value != 'second':\n raise AssertionError(sys.argv)\nprint(value)\n",
    )?;
    crate::support::make_executable(&payload)?;
    write_posix_fixture_command(
        &adapter_launcher,
        "#!/bin/sh\ncount=0\ntest ! -f \"$FIXTURE_ADAPTER_COUNTER\" || count=$(cat \"$FIXTURE_ADAPTER_COUNTER\")\ncount=$((count + 1))\nprintf '%s\\n' \"$count\" > \"$FIXTURE_ADAPTER_COUNTER\"\ntest \"$count\" != 1 || sleep 1\nexec \"$FIXTURE_ADAPTER_PYTHON\" \"$@\"\n",
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
        .env_path("CODEXY_FIXTURE_GH_ADAPTER_LAUNCHER", &adapter_launcher)
        .env_path("FIXTURE_ADAPTER_COUNTER", &counter)
        .env_path(
            "FIXTURE_ADAPTER_PYTHON",
            fixture_script_interpreter_path(&fixture_github_argv_adapter_path(&script))?,
        )
        .env("GITHUB_REPOSITORY", "eunsoogi/codexy")
        .output()?;
    assert!(
        output.status.success(),
        "stdout: {} stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = normalize_fixture_text(&String::from_utf8(output.stdout)?);
    let mut values = stdout.lines().collect::<Vec<_>>();
    values.sort_unstable();
    assert_eq!(values, ["first", "second"]);
    Ok(())
}
