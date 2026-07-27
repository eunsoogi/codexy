#[allow(unused_imports)]
use std::process::Command;

#[path = "release_archive/archive_process.rs"]
mod archive_process;
#[allow(unused_imports)]
pub(crate) use archive_process::{create_archive, create_archive_with_commands};

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

#[allow(dead_code)]
pub(crate) fn assert_archive_scanner_contract(script: &str, checker: &str) {
    assert_structured_literals(
        script,
        "archive scanner behavior",
        &[
            "rg -a -n",
            "grep -a -Hn",
            "runtime/*.bin",
            "!**/hooks/policy-inventory.json",
            "hooks/policy-inventory.json\" ! -name '*.md'",
            "archive policy inventory hygiene scan failed",
            "key not in {\"source\", \"text\"}",
            "! -name '*.md'",
            "! -name '*.txt'",
            "command -v python3",
            "inspect-mcp-entrypoints",
            "shasum -a 256",
            "rg or grep is required",
            "hygiene scan failed",
            "duplicate archive entries",
            "unexpected runtime artifact",
            "unsafe archive path",
        ],
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

#[allow(dead_code)]
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
    for exact_line in ["legacy-public)", "candidate-proven)"] {
        assert!(workflow_lines(assembly).any(|line| line == exact_line));
    }
    for binary in [
        "plugins/codexy/runtime/codexy-mcp-lsp-darwin-arm64.bin",
        "plugins/codexy/runtime/codexy-mcp-codegraph-darwin-arm64.bin",
        "plugins/codexy/runtime/codexy-mcp-lsp-linux-x86_64.bin",
        "plugins/codexy/runtime/codexy-mcp-codegraph-linux-x86_64.bin",
    ] {
        assert!(
            assembly
                .split_whitespace()
                .map(|token| token.trim_end_matches('\\'))
                .any(|token| token == binary)
        );
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

pub(crate) fn copy_tree(source: &std::path::Path, target: &std::path::Path) -> std::io::Result<()> {
    std::fs::create_dir_all(target)?;
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());
        if source_path.is_dir() {
            if entry.file_name() != "runtime" {
                copy_tree(&source_path, &target_path)?;
            }
        } else {
            std::fs::copy(source_path, target_path)?;
        }
    }
    Ok(())
}

pub(crate) fn make_executable(path: &std::path::Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = std::fs::metadata(path)?.permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(path, permissions)?;
    }
    Ok(())
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
    let plugin_root = root.join("plugins/codexy");
    copy_tree(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy"),
        &plugin_root,
    )?;
    let runtime = plugin_root.join("runtime");
    std::fs::create_dir_all(&runtime)?;
    let host_platform = match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "aarch64") => "darwin-arm64",
        ("linux", "x86_64") => "linux-x86_64",
        (os, architecture) => {
            return Err(std::io::Error::other(format!(
                "unsupported test host platform: {os}-{architecture}"
            )));
        }
    };
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
                std::fs::write(&path, header.repeat(1024))?;
            }
            make_executable(&path)?;
        }
    }
    Ok(plugin_root)
}
