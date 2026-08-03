use crate::support::{
    executable_path::executable_path_in,
    fixture_hook_path::{
        hook_fixture_model_input_for_platform, modeled_path_token, project_modeled_paths,
    },
    fixture_hook_path_windows::{fixture_path_cache_key, native_shell_fixture_path_with},
    fixture_path::{windows_fixture_environment_value, windows_to_posix_fixture_path},
    fixture_probe::{FixtureProbe, fixture_probe_path, install_fixture_probe},
};

#[test]
fn discovery_resolves_pathext_candidates_and_rejects_missing_or_ambiguous_inputs()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let candidate = temp.path().join("rg.EXE");
    std::fs::write(&candidate, b"fixture")?;
    let path = std::env::join_paths([temp.path()])?;
    assert_eq!(
        executable_path_in("rg", &path, std::ffi::OsStr::new(".EXE"))?,
        candidate
    );
    assert_eq!(
        executable_path_in("missing", &path, std::ffi::OsStr::new(".EXE")),
        Err("required command missing: missing".to_owned())
    );
    assert_eq!(
        executable_path_in("rg", &path, std::ffi::OsStr::new(".EXE;.exe")),
        Err("ambiguous PATHEXT entry: .exe".to_owned())
    );
    Ok(())
}

#[test]
fn modeled_path_projection_touches_only_declared_operands() {
    let command =
        "sudo -D /c/work/foreign git status && ln -s /usr/bin/printf left && printf C:unrelated";
    assert_eq!(
        project_modeled_paths(command, |path| match path {
            "/c/work/foreign" => Ok(r"C:\work\foreign".into()),
            "/usr/bin/printf" => Ok(r"C:\Git\usr\bin\printf".into()),
            other => Err(other.into()),
        }),
        Ok("sudo -D 'C:\\work\\foreign' git status && ln -s 'C:\\Git\\usr\\bin\\printf' left && printf C:unrelated".into()),
    );
}

#[test]
fn modeled_path_projection_converts_copy_sources_without_rewriting_destinations() {
    let command = "cp -fP /usr/bin/printf left && printf /usr/bin/printf";
    assert_eq!(
        project_modeled_paths(command, |path| match path {
            "/usr/bin/printf" => Ok(r"C:\Git\usr\bin\printf.exe".into()),
            other => Err(other.into()),
        }),
        Ok("cp -fP 'C:\\Git\\usr\\bin\\printf.exe' left && printf /usr/bin/printf".into()),
    );
}

#[test]
fn modeled_path_projection_does_not_treat_scoped_commands_as_copy_operations() {
    let command = "scp /usr/bin/printf remote:";
    assert_eq!(
        project_modeled_paths(command, |_| Err("copy source must stay unprojected".into())),
        Ok(command.into()),
    );
}

#[test]
fn windows_hook_model_input_preserves_native_cwd_and_only_projects_shell_operands() {
    let native_cwd = r"C:\work\owned";
    let command =
        "sudo -D /c/work/foreign git status && ln -s /usr/bin/printf left && printf C:unrelated";
    assert_eq!(
        hook_fixture_model_input_for_platform(command, native_cwd, true, |path| match path {
            "/c/work/foreign" => Ok(r"C:\work\foreign".into()),
            "/usr/bin/printf" => Ok(r"C:\Git\usr\bin\printf".into()),
            other => Err(other.into()),
        }),
        Ok((
            "sudo -D 'C:\\work\\foreign' git status && ln -s 'C:\\Git\\usr\\bin\\printf' left && printf C:unrelated".into(),
            native_cwd.into(),
        )),
    );
    assert!(
        hook_fixture_model_input_for_platform("git status", r"\\server\share", true, |value| {
            Ok(value.to_owned())
        })
        .is_err()
    );
}

#[test]
fn modeled_path_tokens_quote_raw_windows_values_without_touching_non_paths() {
    assert_eq!(
        modeled_path_token(r"C:\work\fixture path", &|_| unreachable!()),
        Ok(Some(r"'C:\work\fixture path'".into())),
    );
    assert_eq!(
        modeled_path_token(r"C:\work\O'Brien", &|_| unreachable!()),
        Ok(Some("'C:\\work\\O'\"'\"'Brien'".into())),
    );
    assert_eq!(
        modeled_path_token("C:relative", &|_| unreachable!()),
        Ok(None)
    );
    assert!(modeled_path_token(r"\\server\share", &|_| unreachable!()).is_err());
}

#[test]
fn native_model_uses_host_identities_for_declared_posix_fixture_paths() {
    let discover = |name: &str| -> Result<String, String> {
        match name {
            "git" => Ok(r"C:\\host\\git.exe".to_owned()),
            "sh" => Ok(r"C:\\host\\sh.exe".to_owned()),
            other => Err(format!("missing {other}")),
        }
    };
    let convert = |path: &str| -> Result<String, String> { Ok(format!(r"C:\\converted\\{path}")) };

    assert_eq!(
        native_shell_fixture_path_with("/usr/bin/git", r"C:\\host\\fixture", discover, convert),
        Ok(r"C:\\host\\git.exe".to_owned())
    );
    assert_eq!(
        native_shell_fixture_path_with("/usr/bin/printf", r"C:\\host\\fixture", discover, convert),
        Ok(r"C:\\host\\sh.exe".to_owned())
    );
    assert_eq!(
        native_shell_fixture_path_with("/var/tmp", r"C:\\host\\fixture", discover, convert),
        Ok(r"C:\\host\\fixture".to_owned())
    );
    assert_eq!(
        native_shell_fixture_path_with("/opt/custom", r"C:\\host\\fixture", discover, convert),
        Ok(r"C:\\converted\\/opt/custom".to_owned())
    );
    assert_eq!(
        native_shell_fixture_path_with(r"\\server\\share", r"C:\\host\\fixture", discover, |_| {
            Err("Windows fixture paths do not support UNC values".to_owned())
        }),
        Err("Windows fixture paths do not support UNC values".to_owned())
    );
}

#[test]
fn fixture_path_cache_key_keeps_fixture_root_context() {
    assert_ne!(
        fixture_path_cache_key("/var/tmp", r"C:\\host\\fixture-a"),
        fixture_path_cache_key("/var/tmp", r"C:\\host\\fixture-b")
    );
    assert_eq!(
        fixture_path_cache_key("/usr/bin/git", r"C:\\host\\fixture-a"),
        fixture_path_cache_key("/usr/bin/git", r"C:\\host\\fixture-b")
    );
}

#[test]
fn windows_fixture_paths_use_the_msys_absolute_path_contract() {
    assert_eq!(
        windows_to_posix_fixture_path("C:\\work\\fixture path"),
        Ok("/c/work/fixture path".into())
    );
    assert_eq!(
        windows_to_posix_fixture_path("D:/runtime/cache"),
        Ok("/d/runtime/cache".into())
    );
    assert_eq!(
        windows_to_posix_fixture_path(r"\\?\D:\runtime\cache"),
        Ok("/d/runtime/cache".into())
    );
    assert_eq!(
        windows_to_posix_fixture_path("C:relative"),
        Err("Windows fixture path must be absolute: C:relative".into())
    );
    assert_eq!(
        windows_to_posix_fixture_path("\\\\server\\share"),
        Err("Windows fixture paths do not support UNC values: \\\\server\\share".into())
    );
    assert_eq!(
        windows_fixture_environment_value("CODEXY_RUNTIME_DIR", "C:\\runtime\\with spaces"),
        Ok("/c/runtime/with spaces".into())
    );
    assert_eq!(
        windows_fixture_environment_value("CODEXY_RUNTIME_PLATFORM", "windows-x86_64"),
        Ok("windows-x86_64".into())
    );
    assert_eq!(
        windows_fixture_environment_value("GIT_DIR", "host-git-dir"),
        Ok("host-git-dir".into())
    );
    assert_eq!(
        windows_fixture_environment_value("GIT_DIR", "C:\\work\\git-dir"),
        Ok("/c/work/git-dir".into())
    );
    assert_eq!(
        windows_fixture_environment_value("GIT_COMMON_DIR", "host-common"),
        Ok("host-common".into())
    );
    assert_eq!(
        windows_fixture_environment_value("GIT_COMMON_DIR", "D:/work/common"),
        Ok("/d/work/common".into())
    );
}

#[test]
fn fixture_probe_preserves_the_requested_platform_artifact_name() {
    let path = std::path::Path::new("runtime/codexy-mcp-lsp-darwin-arm64.bin");
    assert_eq!(fixture_probe_path(path), path);
}

#[test]
fn fixture_probe_preserves_argv_stdout_stderr_and_exit_status()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let probe = install_fixture_probe(
        temp.path().join("argv probe").as_path(),
        FixtureProbe::Arguments,
    )?;
    assert_eq!(probe.logical_path(), temp.path().join("argv probe"));
    let output = probe
        .command()
        .arg("value with spaces")
        .env("CODEXY_FIXTURE_PROBE_STDERR", "stderr mirror")
        .env("CODEXY_FIXTURE_PROBE_EXIT", "23")
        .output()?;
    assert_eq!(output.status.code(), Some(23));
    assert_eq!(String::from_utf8(output.stdout)?, "value with spaces\n");
    assert_eq!(String::from_utf8(output.stderr)?, "stderr mirror\n");
    Ok(())
}
