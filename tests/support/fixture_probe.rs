use std::path::{Path, PathBuf};

use super::make_executable;

pub(crate) enum FixtureProbe {
    Arguments,
}

pub(crate) fn install_fixture_probe(path: &Path, probe: FixtureProbe) -> std::io::Result<PathBuf> {
    let path = fixture_probe_path(path);
    let configuration = match probe {
        FixtureProbe::Arguments => "argv\n".to_owned(),
    };
    #[cfg(windows)]
    if path
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("exe"))
    {
        std::fs::copy(env!("CARGO_BIN_EXE_codexy-fixture-probe"), &path)?;
    } else {
        write_posix_probe(&path, &configuration)?;
    }
    #[cfg(not(windows))]
    write_posix_probe(&path, &configuration)?;
    std::fs::write(path.with_extension("fixture"), configuration)?;
    make_executable(&path)?;
    Ok(path)
}

fn fixture_probe_path(path: &Path) -> PathBuf {
    #[cfg(windows)]
    {
        return path.to_path_buf();
    }
    #[cfg(not(windows))]
    path.to_path_buf()
}

#[test]
fn fixture_probe_preserves_the_requested_platform_artifact_name() {
    let path = Path::new("runtime/codexy-mcp-lsp-darwin-arm64.bin");
    assert_eq!(fixture_probe_path(path), path);
}

fn write_posix_probe(path: &Path, configuration: &str) -> std::io::Result<()> {
    let mut lines = configuration.lines();
    let script = match lines.next() {
        Some("argv") => "#!/bin/sh\nfor arg do printf '%s\\n' \"$arg\"; done\n[ -z \"${CODEXY_FIXTURE_PROBE_STDERR:-}\" ] || printf '%s\\n' \"$CODEXY_FIXTURE_PROBE_STDERR\" >&2\nexit \"${CODEXY_FIXTURE_PROBE_EXIT:-0}\"\n".to_owned(),
        _ => String::new(),
    };
    std::fs::write(path, script)
}

#[test]
fn fixture_probe_preserves_argv_stdout_stderr_and_exit_status()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let probe = install_fixture_probe(
        temp.path().join("argv probe").as_path(),
        FixtureProbe::Arguments,
    )?;
    let output = std::process::Command::new(probe)
        .arg("value with spaces")
        .env("CODEXY_FIXTURE_PROBE_STDERR", "stderr mirror")
        .env("CODEXY_FIXTURE_PROBE_EXIT", "23")
        .output()?;
    assert_eq!(output.status.code(), Some(23));
    assert_eq!(String::from_utf8(output.stdout)?, "value with spaces\n");
    assert_eq!(String::from_utf8(output.stderr)?, "stderr mirror\n");
    Ok(())
}
