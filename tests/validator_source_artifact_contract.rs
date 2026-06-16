use std::process::Command;

#[test]
fn validator_cli_rejects_source_plugin_with_generated_bin_directory()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = copy_plugin_to(temp.path())?;
    let bin_dir = plugin_root.join("bin");
    std::fs::create_dir_all(&bin_dir)?;
    std::fs::write(
        bin_dir.join("codexy-mcp-lsp-linux-x86_64.bin"),
        b"generated",
    )?;

    let output = validate_source_plugin(&plugin_root)?;

    assert!(
        !output.status.success(),
        "source validation should reject generated bin directories"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("plugins/codexy/bin must not be tracked in the source plugin tree")
            || String::from_utf8_lossy(&output.stderr)
                .contains("bin must not be tracked in the source plugin tree"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_cli_rejects_source_plugin_with_generated_runtime_directory()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = copy_plugin_to(temp.path())?;
    let runtime_dir = plugin_root.join("runtime");
    std::fs::create_dir_all(&runtime_dir)?;
    std::fs::write(
        runtime_dir.join("codexy-mcp-codegraph-linux-x86_64.bin"),
        b"generated",
    )?;

    let output = validate_source_plugin(&plugin_root)?;

    assert!(
        !output.status.success(),
        "source validation should reject generated runtime directories"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("runtime must not be tracked in the source plugin tree"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

fn validate_source_plugin(
    plugin_root: &std::path::Path,
) -> Result<std::process::Output, Box<dyn std::error::Error>> {
    Ok(Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--plugin-root",
            plugin_root.to_str().ok_or("plugin root path")?,
            "--check",
        ])
        .output()?)
}

fn copy_plugin_to(temp_root: &std::path::Path) -> std::io::Result<std::path::PathBuf> {
    let plugin_root = temp_root.join("codexy");
    copy_dir(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("plugins/codexy")
            .as_path(),
        &plugin_root,
    )?;
    Ok(plugin_root)
}

fn copy_dir(source: &std::path::Path, target: &std::path::Path) -> std::io::Result<()> {
    std::fs::create_dir_all(target)?;
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());
        if source_path.is_dir() {
            copy_dir(&source_path, &target_path)?;
        } else {
            std::fs::copy(source_path, target_path)?;
        }
    }
    Ok(())
}
