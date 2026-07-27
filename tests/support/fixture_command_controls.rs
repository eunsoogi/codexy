use super::{
    FixtureCommand, fixture_script_launcher, windows_fixture_companion,
    windows_static_python_fixture,
};

#[test]
fn fixture_script_launcher_maps_only_the_inventoryed_windows_shebangs() {
    assert_eq!(
        fixture_script_launcher(true, b"#!/bin/sh\necho fixture\n"),
        Ok(Some("sh"))
    );
    assert_eq!(
        fixture_script_launcher(true, b"#!/bin/sh\r\necho fixture\r\n"),
        Ok(Some("sh"))
    );
    assert_eq!(
        fixture_script_launcher(true, b"#!/usr/bin/env bash\necho fixture\n"),
        Ok(Some("bash"))
    );
    assert_eq!(
        fixture_script_launcher(true, b"#!/usr/bin/env python3\nprint('fixture')\n"),
        Ok(Some("python"))
    );
    assert_eq!(fixture_script_launcher(true, b"MZ\x90\0"), Ok(None));
    assert_eq!(
        fixture_script_launcher(false, b"#!/bin/sh\necho fixture\n"),
        Ok(None)
    );
}

#[test]
fn fixture_script_launcher_rejects_unknown_and_malformed_windows_shebangs() {
    assert_eq!(
        fixture_script_launcher(true, b"#!/usr/bin/env ruby\nputs 'fixture'\n"),
        Err("unsupported fixture script shebang: #!/usr/bin/env ruby".to_owned())
    );
    assert_eq!(
        fixture_script_launcher(true, b"#!/bin/sh-invalid\necho fixture\n"),
        Err("unsupported fixture script shebang: #!/bin/sh-invalid".to_owned())
    );
    assert_eq!(
        fixture_script_launcher(true, b"#!\nfixture\n"),
        Err("malformed fixture script shebang".to_owned())
    );
}

#[test]
fn windows_fixture_companion_selects_only_a_paired_shell_entrypoint()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let shell = temp.path().join("fixture.sh");
    let command = temp.path().join("fixture.cmd");
    std::fs::write(&shell, "#!/bin/sh\n")?;
    assert_eq!(windows_fixture_companion(&shell), None);
    std::fs::write(&command, "@echo off\n")?;
    assert_eq!(windows_fixture_companion(&shell), Some(command));
    assert_eq!(
        windows_fixture_companion(&temp.path().join("fixture.py")),
        None
    );
    Ok(())
}

#[test]
fn windows_static_python_fixture_requires_the_supported_paired_dispatch_contract()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let shell = temp.path().join("codexy-admission.sh");
    let command = temp.path().join("codexy-admission.cmd");
    let python = temp.path().join("codexy-admission.py");
    std::fs::write(&shell, "#!/bin/sh\n")?;
    std::fs::write(
        &command,
        "py -3 -I -B \"%~dp0codexy-admission.py\" --event \"%event%\"\n",
    )?;
    assert_eq!(windows_static_python_fixture(&shell), None);
    std::fs::write(&python, "print('fixture')\n")?;
    assert_eq!(windows_static_python_fixture(&shell), Some(python));
    std::fs::write(&command, "py -3 fixture.py\n")?;
    assert_eq!(windows_static_python_fixture(&shell), None);
    Ok(())
}

#[cfg(unix)]
#[test]
fn fixture_command_preserves_script_arguments_cwd_stdin_and_exit_status()
-> Result<(), Box<dyn std::error::Error>> {
    use std::io::Write as _;

    let temp = tempfile::tempdir()?;
    let working_directory = temp.path().join("working directory");
    std::fs::create_dir(&working_directory)?;
    let script = working_directory.join("fixture script");
    std::fs::write(
        &script,
        "#!/bin/sh\ninput=$(cat)\nprintf '%s\\n%s\\n%s\\n' \"$PWD\" \"$1\" \"$input\"\nexit 17\n",
    )?;
    super::super::make_executable(&script)?;

    let mut child = FixtureCommand::new(&script)
        .arg("argument with spaces")
        .current_dir(&working_directory)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()?;
    child
        .stdin
        .as_mut()
        .ok_or("fixture stdin")?
        .write_all(b"stdin with spaces")?;
    let output = child.wait_with_output()?;

    assert_eq!(output.status.code(), Some(17));
    assert_eq!(
        String::from_utf8(output.stdout)?,
        format!(
            "{}\nargument with spaces\nstdin with spaces\n",
            working_directory.canonicalize()?.display()
        )
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn fixture_command_preserves_python_arguments_stdout_stderr_and_exit_status()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let script = temp.path().join("fixture.py");
    std::fs::write(
        &script,
        "#!/usr/bin/env python3\nimport sys\nprint(sys.argv[1])\nprint('stderr mirror', file=sys.stderr)\nsys.exit(23)\n",
    )?;
    super::super::make_executable(&script)?;

    let output = FixtureCommand::new(&script)
        .arg("argument with spaces")
        .current_dir(temp.path())
        .output()?;

    assert_eq!(output.status.code(), Some(23));
    assert_eq!(String::from_utf8(output.stdout)?, "argument with spaces\n");
    assert_eq!(String::from_utf8(output.stderr)?, "stderr mirror\n");
    Ok(())
}

#[cfg(unix)]
#[test]
fn materialized_script_preserves_source_relative_scanner_diagnostics()
-> Result<(), Box<dyn std::error::Error>> {
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
        "#!/bin/sh\nroot=$(CDPATH= cd -- \"$(dirname -- \"$0\")/..\" && pwd)\n[ -f \"$root/repo-state\" ] || { echo 'missing shared repo state' >&2; exit 2; }\nexec \"$(dirname -- \"$0\")/scanner\" \"$1\"\n",
    )?;
    std::fs::write(
        &scanner,
        "#!/bin/sh\nprintf '%s scanner: archive contains a secret or local path\\n' \"$1\" >&2\nexit 1\n",
    )?;
    for path in [&source, &scanner] {
        super::super::make_executable(path)?;
    }

    super::super::materialize_lf_text_fixture(&source, &target)?;

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
