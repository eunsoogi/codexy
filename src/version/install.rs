use std::fs;

use anyhow::{Context as _, Result, bail};

use crate::paths::repo_root;

const PATH: &str = "install";
const COMMAND_PREFIX: &str = "exec uvx --from getcodexy==";
const COMMAND_SUFFIX: &str = " codexy-update --pre-session \"$@\"";

pub(super) fn check_version(version: &str) -> Result<()> {
    let text =
        fs::read_to_string(repo_root()?.join(PATH)).context("missing required file: install")?;
    let (_, command) = executable_command(&text)?;
    let expected = expected_command(version);
    if command.trim() == expected {
        Ok(())
    } else {
        bail!("install must pin {expected}")
    }
}

pub(super) fn set_version(version: &str) -> Result<()> {
    let path = repo_root()?.join(PATH);
    let text = fs::read_to_string(&path).context("missing required file: install")?;
    let (index, command) = executable_command(&text)?;
    let indentation = &command[..command.len() - command.trim_start().len()];
    let mut lines: Vec<String> = text.lines().map(str::to_owned).collect();
    lines[index] = format!("{indentation}{}", expected_command(version));
    let trailing_newline = text.ends_with('\n');
    let mut updated = lines.join("\n");
    if trailing_newline {
        updated.push('\n');
    }
    fs::write(path, updated).context("writing install")
}

fn expected_command(version: &str) -> String {
    format!("{COMMAND_PREFIX}{version}{COMMAND_SUFFIX}")
}

fn executable_command(text: &str) -> Result<(usize, &str)> {
    let commands: Vec<_> = text
        .lines()
        .enumerate()
        .filter(|(_, line)| line.trim_start().starts_with("exec "))
        .collect();
    let [(index, command)] = commands.as_slice() else {
        bail!("install must contain exactly one executable command")
    };
    if !command.trim().starts_with(COMMAND_PREFIX) || !command.trim().ends_with(COMMAND_SUFFIX) {
        bail!("install executable command must invoke codexy-update --pre-session")
    }
    Ok((*index, *command))
}
