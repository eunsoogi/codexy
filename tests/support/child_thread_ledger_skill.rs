use std::path::{Path, PathBuf};
use std::process::{ExitStatus, Output};

use codexy_runtime::paths;
use codexy_runtime::validation::{self, Mode};

#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;
#[cfg(windows)]
use std::os::windows::process::ExitStatusExt;

pub(crate) struct PluginFixture {
    _temp: tempfile::TempDir,
    root: PathBuf,
}

impl PluginFixture {
    pub(crate) fn root(&self) -> &Path {
        &self.root
    }

    pub(crate) fn reset_file(&self, relative: &Path) -> std::io::Result<()> {
        if !relative.is_relative()
            || relative
                .components()
                .any(|component| matches!(component, std::path::Component::ParentDir))
        {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "fixture reset path must be relative",
            ));
        }
        let source = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("plugins/codexy")
            .join(relative);
        std::fs::copy(source, self.root.join(relative)).map(|_| ())
    }
}

pub(crate) fn plugin_fixture() -> TestResult<PluginFixture> {
    let temp = tempfile::tempdir()?;
    let root = temp.path().join("codexy");
    super::copy_dir(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy"),
        &root,
    )?;
    Ok(PluginFixture { _temp: temp, root })
}

pub(crate) fn copy_plugin_fixture() -> TestResult<(tempfile::TempDir, PathBuf)> {
    let fixture = plugin_fixture()?;
    Ok((fixture._temp, fixture.root))
}

pub(crate) fn validator(
    plugin_root: &Path,
    mode: &str,
) -> Result<std::process::Output, Box<dyn std::error::Error>> {
    validator_in_process(plugin_root, mode)
}

pub(crate) fn validator_instruction_policy(
    plugin_root: &Path,
) -> Result<Output, Box<dyn std::error::Error>> {
    validator_in_process_mode(plugin_root, Mode::InstructionPolicy)
}

pub(crate) fn validator_routing(plugin_root: &Path) -> Result<Output, Box<dyn std::error::Error>> {
    validator_in_process_mode(plugin_root, Mode::OrchestrationRouting)
}

pub(crate) fn validator_in_process(
    plugin_root: &Path,
    mode: &str,
) -> Result<Output, Box<dyn std::error::Error>> {
    let mode = match mode {
        "--check" => Mode::All,
        "--check-mcp" => Mode::Mcp,
        "--check-roles" => Mode::Roles,
        _ => return Err(format!("unsupported in-process validation mode: {mode}").into()),
    };
    validator_in_process_mode(plugin_root, mode)
}

fn validator_in_process_mode(
    plugin_root: &Path,
    mode: Mode,
) -> Result<Output, Box<dyn std::error::Error>> {
    let errors = validation::errors(plugin_root, mode);
    let stderr = errors
        .iter()
        .map(|error| format!("error: {error}"))
        .chain((!errors.is_empty()).then(|| {
            format!(
                "Error: plugin validation failed with {} error(s)",
                errors.len()
            )
        }))
        .collect::<Vec<_>>()
        .join("\n");
    Ok(Output {
        status: exit_status(errors.is_empty()),
        stdout: errors
            .is_empty()
            .then(|| {
                format!(
                    "plugin config validation ok: {}\n",
                    paths::display_relative(plugin_root)
                )
            })
            .unwrap_or_default()
            .into_bytes(),
        stderr: (!stderr.is_empty())
            .then(|| format!("{stderr}\n"))
            .unwrap_or_default()
            .into_bytes(),
    })
}

fn exit_status(success: bool) -> ExitStatus {
    #[cfg(unix)]
    return ExitStatus::from_raw(if success { 0 } else { 1 << 8 });
    #[cfg(windows)]
    ExitStatus::from_raw(i32::from(!success))
}

pub(crate) fn stderr(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
