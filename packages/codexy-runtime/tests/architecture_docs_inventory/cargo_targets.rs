use std::{collections::BTreeSet, env, path::Path, process::Command};

const INVENTORY_TESTS: usize = 6;
const RUNTIME_MANIFEST: &str = "packages/codexy-runtime/Cargo.toml";
const SELECTION_PROBE: &str = "CODEXY_ARCHITECTURE_INVENTORY_SELECTION_PROBE";

#[derive(Debug)]
pub(super) struct DocumentedCommand {
    arguments: Vec<String>,
    target: String,
}

pub(super) fn selection_probe() -> bool {
    env::var_os(SELECTION_PROBE).is_some()
}

pub(super) fn validate(root: &Path, guide: &str) -> Result<DocumentedCommand, String> {
    let command = documented_command(guide)?;
    if metadata_targets(root)?.contains(&command.target) {
        Ok(command)
    } else {
        Err(format!(
            "documented Cargo test target is undeclared: {}",
            command.target
        ))
    }
}

pub(super) fn assert_executes_inventory_tests(
    root: &Path,
    command: &DocumentedCommand,
) -> Result<(), String> {
    let output = Command::new("cargo")
        .args(&command.arguments)
        .current_dir(root)
        .env(SELECTION_PROBE, "1")
        .output()
        .map_err(|error| error.to_string())?;
    let transcript = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    if output.status.success()
        && transcript.contains(&format!("running {INVENTORY_TESTS} tests"))
        && transcript.contains(&format!("test result: ok. {INVENTORY_TESTS} passed;"))
    {
        Ok(())
    } else {
        Err(format!(
            "documented Cargo command did not execute {INVENTORY_TESTS} inventory tests: {transcript}"
        ))
    }
}

fn documented_command(guide: &str) -> Result<DocumentedCommand, String> {
    let mut commands = guide
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with("cargo test "))
        .map(parse_command)
        .collect::<Result<Vec<_>, _>>()?;
    match commands.len() {
        1 => commands
            .pop()
            .ok_or("documented Cargo command disappeared".to_owned()),
        0 => Err("architecture guide has no documented cargo test command".to_owned()),
        _ => Err("architecture guide has multiple documented cargo test commands".to_owned()),
    }
}

fn parse_command(line: &str) -> Result<DocumentedCommand, String> {
    let arguments = line
        .split_whitespace()
        .skip(1)
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let test_flag = arguments
        .iter()
        .position(|argument| argument == "--test")
        .ok_or("documented Cargo command has no --test target")?;
    let target = arguments
        .get(test_flag + 1)
        .filter(|argument| !argument.starts_with('-'))
        .cloned()
        .ok_or("documented Cargo command has an invalid --test target")?;
    let has_filter = arguments
        .get(test_flag + 2)
        .is_some_and(|argument| !argument.starts_with('-'));
    if !has_filter {
        return Err("documented Cargo command has no test filter".to_owned());
    }
    Ok(DocumentedCommand { arguments, target })
}

fn metadata_targets(root: &Path) -> Result<BTreeSet<String>, String> {
    let output = Command::new("cargo")
        .args([
            "metadata",
            "--manifest-path",
            RUNTIME_MANIFEST,
            "--locked",
            "--no-deps",
            "--format-version",
            "1",
        ])
        .current_dir(root)
        .output()
        .map_err(|error| error.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).into_owned());
    }
    let metadata: serde_json::Value =
        serde_json::from_slice(&output.stdout).map_err(|error| error.to_string())?;
    let package = metadata["packages"]
        .as_array()
        .and_then(|packages| {
            packages.iter().find(|package| {
                package["manifest_path"].as_str().is_some_and(|path| path.ends_with(RUNTIME_MANIFEST))
            })
        })
        .ok_or("runtime package metadata is missing")?;
    let targets = package["targets"]
        .as_array()
        .ok_or("runtime package metadata has no targets")?
        .iter()
        .filter(|target| {
            target["kind"]
                .as_array()
                .is_some_and(|kinds| kinds.iter().any(|kind| kind == "test"))
        })
        .filter_map(|target| target["name"].as_str().map(str::to_owned))
        .collect::<BTreeSet<_>>();
    Ok(targets)
}
