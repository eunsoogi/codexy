use super::wrapper_copy::is_generated_fixture_directory;
#[allow(unused_imports)]
use std::process::Command;
#[path = "release_archive/archive_entry.rs"]
pub(crate) mod archive_entry;
#[path = "release_archive/archive_evidence.rs"]
pub(crate) mod archive_evidence;
#[path = "release_archive/archive_process.rs"]
mod archive_process;
#[allow(unused_imports)]
pub(crate) use archive_process::{create_archive, create_archive_with_commands};
pub(crate) fn inspect_archive(
    archive: &std::path::Path,
    plugin_root: &std::path::Path,
    path: Option<&std::path::Path>,
) -> std::io::Result<std::process::Output> {
    let mut command = crate::support::FixtureCommand::new(
        codexy_runtime::paths::repository_root().join("scripts/inspect-release-archive"),
    );
    command.arg_path(archive).arg_path(plugin_root);
    if let Some(path) = path {
        command.env("PATH", path);
    }
    command.output()
}
pub(crate) fn assert_structured_literals(text: &str, rule_id: &str, required: &[&str]) {
    let missing: Vec<_> = required
        .iter()
        .filter(|literal| !text.contains(**literal))
        .collect();
    assert!(
        missing.is_empty(),
        "structured contract {rule_id} is missing required literals {missing:?}"
    );
}

pub(crate) fn assert_structured_absent_literals(text: &str, rule_id: &str, forbidden: &[&str]) {
    let present: Vec<_> = forbidden
        .iter()
        .filter(|literal| text.contains(**literal))
        .collect();
    assert!(
        present.is_empty(),
        "structured contract {rule_id} contains forbidden literals {present:?}"
    );
}
pub(crate) fn assert_archive_scanner_contract(script: &str, entries: &str, checker: &str) {
    assert_structured_literals(
        script,
        "archive scanner behavior",
        &[
            "rg -a -n",
            "grep -a -Hn",
            "runtime/*.bin",
            "! -name '*.md'",
            "! -name '*.txt'",
            "command -v python3",
            "inspect-mcp-entrypoints",
            "shasum -a 256",
            "rg or grep is required",
            "hygiene scan failed",
            "unexpected runtime artifact",
            "check-release-archive-entries",
        ],
    );
    assert_structured_literals(
        entries,
        "archive entry checker behavior",
        &["duplicate archive entries", "unsafe archive path"],
    );
    assert_structured_literals(
        checker,
        "MCP response checker behavior",
        &[
            "invalid JSON-RPC version for response id",
            "set(responses) != {1, 2}",
        ],
    );
}
pub(crate) fn assert_runtime_workflow_contract(workflow: &str, archive_inspector: &str) {
    let workflow: serde_yaml::Value =
        serde_yaml::from_str(workflow).expect("runtime workflow YAML");
    let job = &workflow["jobs"]["verify-selected-package"];
    let matrix = job["strategy"]["matrix"]["include"]
        .as_sequence()
        .expect("platform matrix");
    assert_eq!(matrix.len(), 2);
    assert_eq!(matrix[0]["platform"], "linux-x86_64");
    assert_eq!(matrix[1]["platform"], "darwin-arm64");
    let assembly = workflow_run(
        job,
        "Assemble state-aware marketplace package without rebuilding",
    );
    for marker in ["legacy-public)", "candidate-proven)"] {
        assert_eq!(
            workflow_lines(assembly)
                .filter(|line| *line == marker)
                .count(),
            1
        );
    }
    let legacy = workflow_branch(assembly, "legacy-public)");
    for binary in [
        "plugins/codexy-devtools/runtime/codexy-mcp-lsp-darwin-arm64.bin",
        "plugins/codexy-devtools/runtime/codexy-mcp-codegraph-darwin-arm64.bin",
        "plugins/codexy-devtools/runtime/codexy-mcp-lsp-linux-x86_64.bin",
        "plugins/codexy-devtools/runtime/codexy-mcp-codegraph-linux-x86_64.bin",
    ] {
        assert!(workflow_lines(legacy).any(|line| {
            line.split_whitespace()
                .next()
                .is_some_and(|word| word.trim_end_matches(';') == binary)
        }));
    }
    let candidate = workflow_branch(assembly, "candidate-proven)");
    for line in [
        "scripts/materialize-runtime-release-archive dist/selected.tar.gz dist/codexy-marketplace-plugin.tar.gz",
        "scripts/inspect-release-archive dist/codexy-marketplace-plugin.tar.gz final-inspect/plugins/codexy-devtools public-release",
    ] {
        assert!(workflow_lines(candidate).any(|current| current == line));
    }
    assert!(workflow_lines(assembly).any(|line| line
        == "scripts/inspect-release-archive dist/codexy-marketplace-plugin.tar.gz \"$staged\""));
    assert!(
        archive_inspector
            .lines()
            .map(str::trim)
            .any(|line| line == "\"$response_checker\" \"$response_file\" \"$server\"")
    );
}
fn workflow_run<'a>(job: &'a serde_yaml::Value, name: &str) -> &'a str {
    job["steps"]
        .as_sequence()
        .and_then(|steps| steps.iter().find(|step| step["name"] == name))
        .and_then(|step| step["run"].as_str())
        .expect("workflow step")
}
fn workflow_lines(run: &str) -> impl Iterator<Item = &str> {
    run.lines().map(str::trim).filter(|line| !line.is_empty())
}
fn workflow_branch<'a>(assembly: &'a str, marker: &str) -> &'a str {
    let mut start = None;
    let mut offset = 0;
    for line in assembly.split_inclusive('\n') {
        if line.trim() == marker {
            assert!(
                start.replace(offset + line.len()).is_none(),
                "duplicate workflow package branch"
            );
        }
        offset += line.len();
    }
    assembly[start.expect("workflow package branch")..]
        .split(";;")
        .next()
        .expect("workflow package branch")
}
pub(crate) fn copy_tree(source: &std::path::Path, target: &std::path::Path) -> std::io::Result<()> {
    std::fs::create_dir_all(target)?;
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());
        if source_path.is_dir() {
            if entry.file_name() != "runtime" && !is_generated_fixture_directory(&source_path) {
                copy_tree(&source_path, &target_path)?;
            }
        } else {
            std::fs::copy(&source_path, &target_path)?;
            if std::fs::read(&source_path)?.starts_with(b"#!") {
                crate::support::make_executable(&target_path)?;
            }
        }
    }
    Ok(())
}
pub(crate) use crate::support::make_executable;
pub(crate) fn governed_archive_mode(
    is_windows: bool,
    is_governed_wrapper: bool,
    _source_mode: u32,
) -> Option<u32> {
    (is_windows && is_governed_wrapper).then_some(0o755)
}
pub(crate) fn fixture_host_platform(os: &str, architecture: &str) -> std::io::Result<&'static str> {
    match (os, architecture) {
        ("macos", "aarch64") => Ok("darwin-arm64"),
        ("linux", "x86_64") => Ok("linux-x86_64"),
        ("windows", "x86_64") => Ok("windows-x86_64"),
        _ => Err(std::io::Error::other(format!(
            "unsupported test host platform: {os}-{architecture}"
        ))),
    }
}
pub(crate) fn complete_plugin_fixture(
    root: &std::path::Path,
) -> std::io::Result<std::path::PathBuf> {
    complete_plugin_fixture_with_runtime(root, true)
}
pub(crate) fn complete_plugin_fixture_with_stubbed_runtime(
    root: &std::path::Path,
) -> std::io::Result<std::path::PathBuf> {
    complete_plugin_fixture_with_runtime(root, false)
}
fn complete_plugin_fixture_with_runtime(
    root: &std::path::Path,
    native_host_runtime: bool,
) -> std::io::Result<std::path::PathBuf> {
    let plugin_root = root.join("plugins/codexy-devtools");
    copy_tree(
        &codexy_runtime::paths::repository_root().join("plugins/codexy-devtools"),
        &plugin_root,
    )?;
    crate::support::materialize_admission_runtime_suite(&plugin_root)?;
    let runtime = plugin_root.join("runtime");
    std::fs::create_dir_all(&runtime)?;
    let host_platform = fixture_host_platform(std::env::consts::OS, std::env::consts::ARCH)?;
    for (server, binary) in [
        ("lsp", env!("CARGO_BIN_EXE_codexy-mcp-lsp")),
        ("codegraph", env!("CARGO_BIN_EXE_codexy-mcp-codegraph")),
    ] {
        for platform in ["darwin-arm64", "linux-x86_64"] {
            let path = runtime.join(format!("codexy-mcp-{server}-{platform}.bin"));
            if native_host_runtime && platform == host_platform {
                std::fs::copy(binary, &path)?;
            } else {
                let header = if platform == "darwin-arm64" {
                    vec![0xcf, 0xfa, 0xed, 0xfe]
                } else {
                    vec![0x7f, b'E', b'L', b'F']
                };
                let mut bytes = header.repeat(1024);
                bytes[16] = u8::from(server == "codegraph");
                std::fs::write(&path, bytes)?;
            }
            crate::support::make_executable(&path)?;
        }
    }
    Ok(plugin_root)
}
