use std::path::{Path, PathBuf};

use super::make_executable;

pub(crate) enum FixtureProbe<'a> {
    Arguments,
    Uname {
        operating_system: &'a str,
        architecture: &'a str,
    },
}

pub(crate) fn install_fixture_probe(
    path: &Path,
    probe: FixtureProbe<'_>,
) -> std::io::Result<PathBuf> {
    let path = fixture_probe_path(path);
    let configuration = match probe {
        FixtureProbe::Arguments => "argv\n".to_owned(),
        FixtureProbe::Uname {
            operating_system,
            architecture,
        } => {
            format!("uname\n{operating_system}\n{architecture}\n")
        }
    };
    #[cfg(windows)]
    std::fs::copy(env!("CARGO_BIN_EXE_codexy-fixture-probe"), &path)?;
    #[cfg(not(windows))]
    write_posix_probe(&path, &configuration)?;
    std::fs::write(path.with_extension("fixture"), configuration)?;
    make_executable(&path)?;
    Ok(path)
}

fn fixture_probe_path(path: &Path) -> PathBuf {
    #[cfg(windows)]
    {
        if path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("exe"))
        {
            return path.to_path_buf();
        }
        return path.with_extension("exe");
    }
    #[cfg(not(windows))]
    path.to_path_buf()
}

#[cfg(not(windows))]
fn write_posix_probe(path: &Path, configuration: &str) -> std::io::Result<()> {
    let mut lines = configuration.lines();
    let script = match lines.next() {
        Some("argv") => "#!/bin/sh\nfor arg do printf '%s\\n' \"$arg\"; done\n[ -z \"${CODEXY_FIXTURE_PROBE_STDERR:-}\" ] || printf '%s\\n' \"$CODEXY_FIXTURE_PROBE_STDERR\" >&2\nexit \"${CODEXY_FIXTURE_PROBE_EXIT:-0}\"\n".to_owned(),
        Some("uname") => format!("#!/bin/sh\ncase \"${{1:-}}\" in -s) printf '%s\\n' '{}' ;; -m) printf '%s\\n' '{}' ;; *) exit 2 ;; esac\n", lines.next().unwrap_or_default(), lines.next().unwrap_or_default()),
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

#[test]
fn fixture_probe_emulates_uname_variants() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let probe = install_fixture_probe(
        temp.path().join("uname").as_path(),
        FixtureProbe::Uname {
            operating_system: "Plan9",
            architecture: "mips64",
        },
    )?;
    assert_eq!(
        std::process::Command::new(&probe)
            .arg("-s")
            .output()?
            .stdout,
        b"Plan9\n"
    );
    assert_eq!(
        std::process::Command::new(probe).arg("-m").output()?.stdout,
        b"mips64\n"
    );
    Ok(())
}
