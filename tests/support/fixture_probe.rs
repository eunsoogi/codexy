use std::path::{Path, PathBuf};

use super::{fixture_command::FixtureCommand, make_executable};

pub(crate) enum FixtureProbe {
    Arguments,
}

pub(crate) struct FixtureProbeExecutable {
    logical_path: PathBuf,
}

impl FixtureProbeExecutable {
    pub(crate) fn logical_path(&self) -> &Path {
        &self.logical_path
    }

    pub(crate) fn command(&self) -> FixtureCommand {
        FixtureCommand::new(&self.logical_path)
    }
}

pub(crate) fn install_fixture_probe(
    path: &Path,
    probe: FixtureProbe,
) -> std::io::Result<FixtureProbeExecutable> {
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
    Ok(FixtureProbeExecutable { logical_path: path })
}

pub(crate) fn fixture_probe_path(path: &Path) -> PathBuf {
    #[cfg(windows)]
    {
        return path.to_path_buf();
    }
    #[cfg(not(windows))]
    path.to_path_buf()
}

fn write_posix_probe(path: &Path, configuration: &str) -> std::io::Result<()> {
    let mut lines = configuration.lines();
    let script = match lines.next() {
        Some("argv") => "#!/bin/sh\nfor arg do printf '%s\\n' \"$arg\"; done\n[ -z \"${CODEXY_FIXTURE_PROBE_STDERR:-}\" ] || printf '%s\\n' \"$CODEXY_FIXTURE_PROBE_STDERR\" >&2\nexit \"${CODEXY_FIXTURE_PROBE_EXIT:-0}\"\n".to_owned(),
        _ => String::new(),
    };
    std::fs::write(path, script)
}
