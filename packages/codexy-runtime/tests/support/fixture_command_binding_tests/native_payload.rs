use super::*;
use crate::support::fixture_github_argv_adapter_path;

#[test]
fn launcher_binding_preserves_native_payload_arguments_across_the_posix_shell()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let script = temp.path().join("release-helper");
    let payload = temp.path().join("gh-payload");
    write_posix_fixture_command(
        &script,
        "#!/bin/sh\ngh repos/eunsoogi/codexy /d/workspace/asset\n",
    )?;
    write_posix_fixture_command(&payload, "#!/bin/sh\nprintf '%s|%s\\n' \"$1\" \"$2\"\n")?;
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
        .env("CODEXY_FIXTURE_GH", &payload)
        .env_path(
            "CODEXY_FIXTURE_GH_LAUNCHER",
            fixture_script_interpreter_path(&payload)?,
        )
        .env_path(
            "CODEXY_FIXTURE_GH_ADAPTER_LAUNCHER",
            fixture_script_interpreter_path(&fixture_github_argv_adapter_path(&script))?,
        )
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
fn launcher_binding_keeps_github_api_operands_logical() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let script = temp.path().join("release-helper");
    let gh = temp.path().join("gh");
    write_posix_fixture_command(
        &script,
        "#!/bin/sh\ngh release view v9.9.9 --repo \"$GITHUB_REPOSITORY\"\ngh api \"repos/$GITHUB_REPOSITORY/releases/tags/v9.9.9\" --raw-field \"state=$FIXTURE_API_VALUE\"\n",
    )?;
    fs::write(
        &gh,
        "#!/usr/bin/env python3\nimport os,sys\nrepo='eunsoogi/codexy'\nassert os.environ['GITHUB_REPOSITORY'] == repo\nassert os.environ['CODEXY_FIXTURE_GH_TRANSPORT'] == '1'\nargs=sys.argv[1:]\nif args[:2] == ['release', 'view']:\n assert args == ['release', 'view', 'v9.9.9', '--repo', repo]\n print('release:' + args[-1])\nelif args[:1] == ['api']:\n assert args == ['api', f'repos/{repo}/releases/tags/v9.9.9', '--raw-field', 'state=literal-api-value']\n print('api:' + args[1])\nelse:\n raise AssertionError(args)\n",
    )?;
    crate::support::make_executable(&gh)?;
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
        .env("CODEXY_FIXTURE_GH", &gh)
        .env_path(
            "CODEXY_FIXTURE_GH_LAUNCHER",
            fixture_script_interpreter_path(&gh)?,
        )
        .env_path(
            "CODEXY_FIXTURE_GH_ADAPTER_LAUNCHER",
            fixture_script_interpreter_path(&fixture_github_argv_adapter_path(&script))?,
        )
        .env("FIXTURE_API_VALUE", "literal-api-value")
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
    Ok(())
}

#[test]
fn launcher_binding_converts_only_declared_native_release_filesystem_operands()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let bin = temp.path().join("bin");
    fs::create_dir(&bin)?;
    let script = temp.path().join("release-helper");
    let gh = temp.path().join("gh");
    let cygpath = bin.join("cygpath");
    write_posix_fixture_command(
        &script,
        "#!/bin/sh\ngh release download v9.9.9 --repo \"$GITHUB_REPOSITORY\" --dir /d/download\ngh release upload v9.9.9 /d/upload\ngh attestation verify /d/artifact --repo \"$GITHUB_REPOSITORY\"\n",
    )?;
    write_posix_fixture_command(
        &cygpath,
        "#!/bin/sh\ntest \"$1\" = -w && test \"$2\" = -- || exit 2\ncase \"$3\" in /d/*) printf 'D:/%s\\n' \"${3#/d/}\" ;; *) exit 3 ;; esac\n",
    )?;
    fs::write(
        &gh,
        "#!/usr/bin/env python3\nimport os,sys\nrepo='eunsoogi/codexy'\nassert os.environ['GITHUB_REPOSITORY'] == repo\nassert os.environ['CODEXY_FIXTURE_GH_TRANSPORT'] == '1'\nargs=sys.argv[1:]\nexpected=[['release','download','v9.9.9','--repo',repo,'--dir','D:/download'],['release','upload','v9.9.9','D:/upload'],['attestation','verify','D:/artifact','--repo',repo]]\nassert args in expected, args\nprint(':'.join(args))\n",
    )?;
    crate::support::make_executable(&gh)?;
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
        .env_path("CODEXY_FIXTURE_GH", &gh)
        .env_path(
            "CODEXY_FIXTURE_GH_LAUNCHER",
            fixture_script_interpreter_path(&gh)?,
        )
        .env_path(
            "CODEXY_FIXTURE_GH_ADAPTER_LAUNCHER",
            fixture_script_interpreter_path(&fixture_github_argv_adapter_path(&script))?,
        )
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
    assert_eq!(
        String::from_utf8(output.stdout)?,
        "release:download:v9.9.9:--repo:eunsoogi/codexy:--dir:D:/download\nrelease:upload:v9.9.9:D:/upload\nattestation:verify:D:/artifact:--repo:eunsoogi/codexy\n"
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
