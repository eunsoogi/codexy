use crate::support::{FixtureCommand, windows_fixture_companion, windows_static_python_fixture};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn fixture_script_launcher_maps_only_the_inventoryed_windows_shebangs() {
    FixtureCommand::assert_fixture_script_launcher_mappings();
}

#[test]
fn fixture_script_launcher_rejects_unknown_and_malformed_windows_shebangs() {
    FixtureCommand::assert_fixture_script_launcher_rejections();
}

#[test]
fn windows_fixture_companion_selects_only_a_paired_shell_entrypoint() -> TestResult {
    let temp = tempfile::tempdir()?;
    let shell = temp.path().join("fixture.sh");
    let command = temp.path().join("fixture.cmd");
    std::fs::write(&shell, "#!/bin/sh\n")?;
    assert_eq!(windows_fixture_companion(&shell), None);
    std::fs::write(&command, "@echo off\n")?;
    assert_eq!(windows_fixture_companion(&shell), Some(command));
    let python = temp.path().join("fixture.py");
    assert_eq!(windows_fixture_companion(&python), None);
    Ok(())
}

#[test]
fn posix_command_mock_uses_the_platform_dispatch_boundary() -> TestResult {
    let temp = tempfile::tempdir()?;
    let bin = temp.path().join("fixture bin with spaces");
    std::fs::create_dir(&bin)?;
    let command = bin.join("git");
    crate::support::write_posix_fixture_command(&command, "#!/bin/sh\nprintf '%s\\n' \"$1\"\n")?;
    #[cfg(windows)]
    {
        assert!(
            command.is_file(),
            "nested sh needs the bare fixture command"
        );
        assert!(!command.with_extension("cmd").exists());
        assert!(!command.with_extension("bat").exists());
    }
    let target = temp.path().join("nested shell target");
    std::fs::write(&target, "#!/bin/sh\ngit 'argument with spaces'\n")?;
    crate::support::make_executable(&target)?;
    let runner = temp.path().join("nested shell runner");
    crate::support::write_posix_fixture_shell_runner(
        &runner,
        "CODEXY_FIXTURE_TARGET",
        &[("git", "CODEXY_FIXTURE_COMMAND")],
    )?;
    let poison_bin = temp.path().join("host bin");
    std::fs::create_dir(&poison_bin)?;
    let poison = temp.path().join("host git ran");
    let poison_git = poison_bin.join("git");
    std::fs::write(
        &poison_git,
        "#!/bin/sh\nprintf '%s\\n' host > \"$CODEXY_FIXTURE_POISON\"\nexit 91\n",
    )?;
    crate::support::make_executable(&poison_git)?;
    let host_path = std::env::var_os("PATH").ok_or("PATH")?;
    let mut paths = vec![poison_bin];
    paths.extend(std::env::split_paths(&host_path));
    let trace = temp.path().join("command trace");
    let mut fixture = FixtureCommand::new(&runner);
    #[cfg(unix)]
    assert_eq!(fixture.get_program(), std::ffi::OsStr::new("/bin/sh"));
    fixture.current_dir(temp.path());
    fixture
        .env_path_list("PATH", paths)
        .env_path("CODEXY_FIXTURE_COMMAND", &command)
        .env_path("CODEXY_FIXTURE_TARGET", &target)
        .env_path("CODEXY_FIXTURE_COMMAND_TRACE", &trace)
        .env_path("CODEXY_FIXTURE_POISON", &poison);
    let output = fixture.output()?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8(output.stdout)?, "argument with spaces\n");
    assert_eq!(std::fs::read_to_string(trace)?, "git\n");
    assert!(!poison.exists(), "same-shell binding reached host git");
    Ok(())
}

#[test]
fn windows_static_python_fixture_requires_the_supported_paired_dispatch_contract() -> TestResult {
    let temp = tempfile::tempdir()?;
    let shell = temp.path().join("codexy-thread-delivery.sh");
    let command = temp.path().join("codexy-thread-delivery.cmd");
    let python = temp.path().join("codexy-thread-delivery.py");
    std::fs::write(&shell, "#!/bin/sh\n")?;
    std::fs::write(
        &command,
        "py -3 -I -B \"%~dp0codexy-thread-delivery.py\" --event \"%event%\"\n",
    )?;
    assert_eq!(windows_static_python_fixture(&shell), None);
    std::fs::write(&python, "print('fixture')\n")?;
    assert_eq!(windows_static_python_fixture(&shell), Some(python));

    let production = codexy_runtime::paths::repository_root()
        .join("plugins/codexy/hooks/codexy-thread-delivery.cmd");
    std::fs::copy(production, &command)?;
    assert_eq!(
        windows_static_python_fixture(&shell),
        Some(shell.with_file_name("codexy-thread-delivery.py"))
    );
    for source in [
        "python -I -B \"%~dp0codexy-thread-delivery.py\" --event \"%event%\"\n",
        "py -3 fixture.py\n",
        "@echo off\nset \"runtime=%SystemRoot%\\py.exe\"\n\"%runtime%\" -3 -I -B \"%~dp0codexy-thread-delivery.py\" --event \"%event%\"\n",
    ] {
        std::fs::write(&command, source)?;
        assert_eq!(windows_static_python_fixture(&shell), None);
    }
    Ok(())
}

#[cfg(windows)]
#[test]
fn absolute_paired_dispatch_uses_the_python_fixture_not_a_posix_shell() -> TestResult {
    let temp = tempfile::tempdir()?;
    let shell = temp.path().join("codexy-thread-delivery.sh");
    let command = temp.path().join("codexy-thread-delivery.cmd");
    let python = temp.path().join("codexy-thread-delivery.py");
    std::fs::write(&shell, "#!/bin/sh\n")?;
    std::fs::copy(
        codexy_runtime::paths::repository_root()
            .join("plugins/codexy/hooks/codexy-thread-delivery.cmd"),
        &command,
    )?;
    std::fs::write(&python, "print('absolute-dispatch')\n")?;

    let fixture = FixtureCommand::new(&shell);
    let program = fixture.get_program().to_string_lossy().to_ascii_lowercase();
    assert!(!program.ends_with("sh") && !program.ends_with("sh.exe"));
    let output = fixture.output()?;
    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout)?, "absolute-dispatch\n");
    Ok(())
}

#[cfg(unix)]
#[test]
fn fixture_command_preserves_python_arguments_stdout_stderr_and_exit_status() -> TestResult {
    let temp = tempfile::tempdir()?;
    let script = temp.path().join("fixture.py");
    std::fs::write(
        &script,
        "#!/usr/bin/env python3\nimport sys\nprint(sys.argv[1])\nprint('stderr mirror', file=sys.stderr)\nsys.exit(23)\n",
    )?;
    crate::support::make_executable(&script)?;
    #[cfg(target_os = "linux")]
    let _open_for_writing = std::fs::File::options().write(true).open(&script)?;

    let output = FixtureCommand::new(&script)
        .arg("argument with spaces")
        .current_dir(temp.path())
        .output()?;

    assert_eq!(output.status.code(), Some(23));
    assert_eq!(String::from_utf8(output.stdout)?, "argument with spaces\n");
    assert_eq!(String::from_utf8(output.stderr)?, "stderr mirror\n");
    Ok(())
}

#[test]
fn large_materialized_script_preserves_source_relative_scanner_diagnostics() -> TestResult {
    let temp = tempfile::tempdir()?;
    let source_root = temp.path().join("source-repo");
    let source_scripts = source_root.join("scripts");
    let target = temp.path().join("materialized/inspect");
    std::fs::create_dir_all(&source_scripts)?;
    std::fs::write(source_root.join("repo-state"), "present\n")?;
    let source = source_scripts.join("inspect");
    let scanner = source_scripts.join("scanner");
    std::fs::write(
        &source,
        format!(
            "#!/bin/sh\n# {}\nroot=$(CDPATH= cd -- \"$(dirname -- \"$0\")/..\" && pwd)\n[ -f \"$root/repo-state\" ] || {{ echo 'missing shared repo state' >&2; exit 2; }}\nexec \"$(dirname -- \"$0\")/scanner\" \"$1\"\n",
            "materialized fixture payload ".repeat(400),
        ),
    )?;
    std::fs::write(
        &scanner,
        "#!/bin/sh\nprintf '%s scanner: archive contains a secret or local path\\n' \"$1\" >&2\nexit 1\n",
    )?;
    for path in [&source, &scanner] {
        crate::support::make_executable(path)?;
    }
    crate::support::materialize_lf_text_fixture(&source, &target)?;

    for backend in ["rg", "grep"] {
        let output = FixtureCommand::new(&target).arg(backend).output()?;
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        assert_eq!(
            String::from_utf8(output.stderr)?,
            format!("{backend} scanner: archive contains a secret or local path\n")
        );
    }
    Ok(())
}
