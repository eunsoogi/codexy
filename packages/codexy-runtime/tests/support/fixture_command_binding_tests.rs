use std::{fs, io::ErrorKind, process::Command};

use crate::support::{
    FixtureArgumentDomain, FixtureCommand, FixtureScriptBinding,
    bind_posix_fixture_script_launchers, bind_posix_fixture_shell_launchers,
    fixture_github_argv_adapter_path, fixture_script_interpreter_path, normalize_fixture_text,
    write_posix_fixture_command, write_posix_fixture_shell_runner,
};

#[path = "fixture_command_binding_tests/native_payload.rs"]
mod native_payload;

#[test]
fn shell_runner_rejects_unsafe_function_identifiers_before_writing()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    for identifier in ["", "9git", "git-name", "if"] {
        let runner = temp.path().join(format!("{identifier}.sh"));
        let error = write_posix_fixture_shell_runner(
            &runner,
            "CODEXY_FIXTURE_TARGET",
            &[(identifier, "CODEXY_FIXTURE_GIT")],
        )
        .expect_err("unsafe shell function identifier must fail closed");
        assert_eq!(error.kind(), ErrorKind::InvalidInput, "{identifier:?}");
        assert!(!runner.exists(), "{identifier:?} wrote a runner");
    }
    Ok(())
}

#[test]
fn shell_runner_matches_the_supported_shell_keyword_boundary()
-> Result<(), Box<dyn std::error::Error>> {
    // Every grammar keyword accepted by the supported sh, including words that are
    // not identifiers. `coproc` is a control: it is a valid function name here.
    const SUPPORTED_SH_KEYWORDS: [&str; 21] = [
        "!", "[[", "]]", "case", "coproc", "do", "done", "elif", "else", "esac", "fi", "for",
        "function", "if", "in", "select", "then", "time", "until", "while", "{",
    ];
    let temp = tempfile::tempdir()?;
    for (index, identifier) in SUPPORTED_SH_KEYWORDS
        .into_iter()
        .chain(["}"].into_iter())
        .enumerate()
    {
        let shell_script = temp.path().join(format!("shell-keyword-{index}"));
        write_posix_fixture_command(
            &shell_script,
            &format!("#!/bin/sh\n{identifier}() {{ :; }}\nexit 99\n"),
        )?;
        let shell_accepts = Command::new("sh")
            .arg("-n")
            .arg(&shell_script)
            .output()?
            .status
            .success();
        let runner = temp.path().join(format!("runner-keyword-{index}"));
        let runner_accepts = write_posix_fixture_shell_runner(
            &runner,
            "CODEXY_FIXTURE_TARGET",
            &[(identifier, "CODEXY_FIXTURE_GIT")],
        )
        .is_ok();
        assert_eq!(
            runner_accepts, shell_accepts,
            "first divergent token: {identifier:?}"
        );
        if !shell_accepts {
            assert!(!runner.exists(), "{identifier:?} wrote a runner");
        }
    }
    Ok(())
}

#[test]
fn shell_runner_executes_the_safe_fixture_identifiers() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let target = temp.path().join("target.sh");
    write_posix_fixture_command(&target, "#!/bin/sh\ngit first\njq second\ngh third\n")?;
    let runner = temp.path().join("runner.sh");
    let bindings = [
        ("git", "CODEXY_FIXTURE_GIT"),
        ("jq", "CODEXY_FIXTURE_JQ"),
        ("gh", "CODEXY_FIXTURE_GH"),
    ];
    write_posix_fixture_shell_runner(&runner, "CODEXY_FIXTURE_TARGET", &bindings)?;
    for (name, _) in bindings {
        let payload = temp.path().join(name);
        write_posix_fixture_command(
            &payload,
            &format!("#!/bin/sh\nprintf '{name}:%s\\n' \"$1\"\n"),
        )?;
    }
    let output = FixtureCommand::new(&runner)
        .env_path("CODEXY_FIXTURE_TARGET", &target)
        .env_path("CODEXY_FIXTURE_GIT", temp.path().join("git"))
        .env_path("CODEXY_FIXTURE_JQ", temp.path().join("jq"))
        .env_path("CODEXY_FIXTURE_GH", temp.path().join("gh"))
        .output()?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout)?,
        "git:first\njq:second\ngh:third\n"
    );
    Ok(())
}

#[test]
fn launcher_binding_uses_the_explicit_interpreter_after_path_is_scrubbed()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let script = temp.path().join("release-helper");
    let gh = temp.path().join("gh");
    let empty_path = temp.path().join("empty-path");
    std::fs::create_dir(&empty_path)?;
    write_posix_fixture_command(&script, "#!/bin/sh\ngh release view\n")?;
    std::fs::write(
        &gh,
        "#!/usr/bin/env python3\nimport sys\nsys.stdout.buffer.write(('gh:' + ' '.join(sys.argv[1:]) + '\\r\\n').encode())\n",
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
        .env("GITHUB_REPOSITORY", "eunsoogi/codexy")
        .env_path(
            "CODEXY_FIXTURE_GH_LAUNCHER",
            fixture_script_interpreter_path(&gh)?,
        )
        .env_path(
            "CODEXY_FIXTURE_GH_ADAPTER_LAUNCHER",
            fixture_script_interpreter_path(&fixture_github_argv_adapter_path(&script))?,
        )
        .env_path("PATH", &empty_path)
        .output()?;
    assert!(
        output.status.success(),
        "stdout: {} stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        normalize_fixture_text(&String::from_utf8(output.stdout)?),
        "gh:release view\n"
    );
    Ok(())
}

#[test]
fn declared_release_child_bindings_replace_once_or_preserve_fixture_source()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let script = temp.path().join("release-helper");
    let launcher = "FIXTURE_POSIX_SHELL";
    let invocation = "scripts/verify-release-settings --require-pypi";
    let child = "scripts/verify-release-settings";
    let source = format!("#!/bin/sh\n{invocation}\n");
    fs::write(&script, &source)?;
    bind_posix_fixture_script_launchers(
        &script,
        launcher,
        "FIXTURE_SCRIPT_ROOT",
        &[FixtureScriptBinding { invocation, child }],
    )?;
    assert_eq!(
        fs::read_to_string(&script)?,
        format!("#!/bin/sh\n\"${launcher}\" \"${{FIXTURE_SCRIPT_ROOT}}/{child}\" --require-pypi\n")
    );

    for (name, candidate, binding, expected_kind) in [
        (
            "absent",
            "#!/bin/sh\nscripts/other-child\n",
            FixtureScriptBinding { invocation, child },
            ErrorKind::InvalidData,
        ),
        (
            "duplicated",
            "#!/bin/sh\nscripts/verify-release-settings --require-pypi\nscripts/verify-release-settings --require-pypi\n",
            FixtureScriptBinding { invocation, child },
            ErrorKind::InvalidData,
        ),
        (
            "mismatched",
            "#!/bin/sh\nscripts/verify-release-settings --require-pypi\n",
            FixtureScriptBinding {
                invocation,
                child: "scripts/other-child",
            },
            ErrorKind::InvalidData,
        ),
        (
            "unsafe",
            "#!/bin/sh\nscripts/../verify-release-settings --require-pypi\n",
            FixtureScriptBinding {
                invocation: "scripts/../verify-release-settings --require-pypi",
                child: "scripts/../verify-release-settings",
            },
            ErrorKind::InvalidInput,
        ),
    ] {
        fs::write(&script, candidate)?;
        let error = bind_posix_fixture_script_launchers(
            &script,
            launcher,
            "FIXTURE_SCRIPT_ROOT",
            &[binding],
        )
        .expect_err("invalid declared child must fail closed");
        assert_eq!(error.kind(), expected_kind, "{name}");
        assert_eq!(
            fs::read_to_string(&script)?,
            candidate,
            "{name} wrote fixture source"
        );
    }
    Ok(())
}
