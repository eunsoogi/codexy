#[cfg(unix)]
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::Path,
    process::Command,
};

#[cfg(unix)]
use serde_yaml::Value;

#[cfg(unix)]
#[test]
fn legacy_public_assembly_projects_runtime_from_detected_archive_root()
-> Result<(), Box<dyn std::error::Error>> {
    let temporary = tempfile::tempdir()?;
    let root = temporary.path().join("legacy public assembly fixture");
    let selected_root = root.join("selected/plugins/codexy/runtime");
    fs::create_dir_all(&selected_root)?;
    let runtime = [
        ("codexy-mcp-lsp-darwin-arm64.bin", b"legacy darwin lsp\n".as_slice()),
        (
            "codexy-mcp-codegraph-darwin-arm64.bin",
            b"legacy darwin codegraph\n".as_slice(),
        ),
        ("codexy-mcp-lsp-linux-x86_64.bin", b"legacy linux lsp\n".as_slice()),
        (
            "codexy-mcp-codegraph-linux-x86_64.bin",
            b"legacy linux codegraph\n".as_slice(),
        ),
    ];
    for (name, bytes) in runtime {
        fs::write(selected_root.join(name), bytes)?;
    }
    fs::create_dir_all(root.join("dist"))?;
    let selected = root.join("dist/selected.tar.gz");
    assert!(
        Command::new("tar")
            .env("COPYFILE_DISABLE", "1")
            .current_dir(root.join("selected"))
            .args(["-czf"])
            .arg(&selected)
            .arg("plugins/codexy")
            .status()?
            .success(),
        "legacy archive creation failed"
    );
    fs::write(root.join("dist/legacy-public"), b"")?;
    fs::create_dir_all(root.join("plugins/codexy-devtools/runtime"))?;
    fs::write(
        root.join("plugins/codexy-devtools/source-marker"),
        b"current source\n",
    )?;
    fs::write(
        root.join("plugins/codexy-devtools/runtime/codexy-mcp-lsp-linux-x86_64.bin"),
        b"stale source runtime\n",
    )?;
    fs::create_dir_all(root.join("scripts"))?;
    let checker = codexy_runtime::paths::repository_root().join("scripts/check-release-archive-entries");
    fs::copy(checker, root.join("scripts/check-release-archive-entries"))?;
    write_executable(
        &root.join("scripts/inspect-release-archive"),
        "#!/bin/sh\nset -eu\nprintf '%s\\n' inspected > inspect-called\n",
    )?;
    write_executable(&root.join("run-assembly"), &format!("#!/bin/sh\nset -eu\n{}", assembly()?))?;

    let host_path = std::env::var_os("PATH").ok_or("PATH")?;
    let mut paths = vec![root.join("scripts")];
    paths.extend(std::env::split_paths(&host_path));
    let output = Command::new(root.join("run-assembly"))
        .current_dir(&root)
        .env("PATH", std::env::join_paths(paths)?)
        .output()?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(fs::read_to_string(root.join("inspect-called"))?, "inspected\n");
    let extracted = root.join("extracted");
    fs::create_dir_all(&extracted)?;
    assert!(
        Command::new("tar")
            .args(["-xzf"])
            .arg(root.join("dist/codexy-marketplace-plugin.tar.gz"))
            .args(["-C"])
            .arg(&extracted)
            .status()?
            .success(),
        "assembled archive extraction failed"
    );
    assert_eq!(
        fs::read(extracted.join(
            "plugins/codexy-devtools/runtime/codexy-mcp-lsp-linux-x86_64.bin",
        ))?,
        b"legacy linux lsp\n"
    );
    assert_eq!(
        fs::read_to_string(extracted.join("plugins/codexy-devtools/source-marker"))?,
        "current source\n"
    );
    Ok(())
}

#[cfg(unix)]
fn assembly() -> Result<String, Box<dyn std::error::Error>> {
    let workflow = codexy_runtime::paths::repository_root()
        .join(".github/workflows/plugin-runtime-binaries.yml");
    let parsed: Value = serde_yaml::from_str(&fs::read_to_string(workflow)?)?;
    parsed["jobs"]["verify-selected-package"]["steps"]
        .as_sequence()
        .and_then(|steps| {
            steps
                .iter()
                .find(|step| step["name"] == "Assemble state-aware marketplace package without rebuilding")
        })
        .and_then(|step| step["run"].as_str())
        .map(str::to_owned)
        .ok_or_else(|| "selected package assembly".into())
}

#[cfg(unix)]
fn write_executable(path: &Path, content: &str) -> Result<(), std::io::Error> {
    fs::write(path, content)?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o755))
}
