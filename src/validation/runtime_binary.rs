use std::path::Path;

use anyhow::{Context as _, Result, bail};

use crate::paths::display_relative;

pub(super) fn artifact_name(server: &str, platform: &str) -> String {
    let extension = if platform == "windows-x86_64" {
        "exe"
    } else {
        "bin"
    };
    format!("codexy-mcp-{server}-{platform}.{extension}")
}

pub(super) fn check(runtime_path: &Path, platform: &str) -> Result<()> {
    let bytes = std::fs::read(runtime_path)
        .with_context(|| format!("reading {}", display_relative(runtime_path)))?;
    let signature_valid = match platform {
        "linux-x86_64" => bytes.starts_with(b"\x7fELF"),
        "darwin-arm64" => {
            bytes.starts_with(&[0xcf, 0xfa, 0xed, 0xfe])
                || bytes.starts_with(&[0xfe, 0xed, 0xfa, 0xcf])
        }
        "windows-x86_64" => is_x86_64_pe(&bytes),
        _ => false,
    };
    if !signature_valid {
        bail!(
            "{} bundled MCP runtime has invalid binary format for {platform}",
            display_relative(runtime_path)
        );
    }
    check_executable(runtime_path, platform)
}

pub(super) fn check_windows_entrypoint_copy(
    plugin_root: &Path,
    server: &str,
    runtime_path: &Path,
) -> Result<()> {
    let entrypoint = plugin_root
        .join("mcp")
        .join(format!("codexy-mcp-{server}.exe"));
    if !entrypoint.is_file() {
        bail!(
            "{} native Windows MCP entrypoint missing for {server}",
            display_relative(&entrypoint)
        );
    }
    reject_link_or_reparse_point(plugin_root, runtime_path)?;
    reject_link_or_reparse_point(plugin_root, &entrypoint)?;
    let runtime = std::fs::read(runtime_path)?;
    let launcher = std::fs::read(&entrypoint)?;
    if launcher != runtime {
        bail!(
            "{} native Windows MCP entrypoint must match {}",
            display_relative(&entrypoint),
            display_relative(runtime_path)
        );
    }
    Ok(())
}

fn is_x86_64_pe(bytes: &[u8]) -> bool {
    if bytes.len() < 0x40 || !bytes.starts_with(b"MZ") {
        return false;
    }
    let offset = u32::from_le_bytes([bytes[0x3c], bytes[0x3d], bytes[0x3e], bytes[0x3f]]) as usize;
    let Some(header) = bytes.get(offset..offset.saturating_add(26)) else {
        return false;
    };
    let optional_header_size = u16::from_le_bytes([header[20], header[21]]) as usize;
    let characteristics = u16::from_le_bytes([header[22], header[23]]);
    let optional_header_end = offset
        .saturating_add(24)
        .saturating_add(optional_header_size);
    header.starts_with(b"PE\0\0")
        && header.get(4..6) == Some(0x8664_u16.to_le_bytes().as_slice())
        && optional_header_size >= 2
        && optional_header_end <= bytes.len()
        && characteristics & 0x0002 != 0
        && characteristics & 0x2000 == 0
        && header.get(24..26) == Some(0x20b_u16.to_le_bytes().as_slice())
}

fn check_executable(runtime_path: &Path, platform: &str) -> Result<()> {
    #[cfg(unix)]
    if platform != "windows-x86_64" {
        use std::os::unix::fs::PermissionsExt as _;

        let mode = runtime_path.metadata()?.permissions().mode();
        if mode & 0o111 == 0 {
            bail!(
                "{} bundled MCP runtime must be executable",
                display_relative(runtime_path)
            );
        }
    }
    #[cfg(not(unix))]
    let _ = (runtime_path, platform);
    Ok(())
}

fn reject_link_or_reparse_point(plugin_root: &Path, path: &Path) -> Result<()> {
    let relative = path.strip_prefix(plugin_root)?;
    let mut current = plugin_root.to_path_buf();
    for component in relative.components() {
        current.push(component);
        let metadata = current.symlink_metadata()?;
        if metadata.file_type().is_symlink() || is_windows_reparse_point(&metadata) {
            bail!(
                "{} native Windows runtime path must not contain a link or reparse point",
                display_relative(&current)
            );
        }
    }
    Ok(())
}

#[cfg(windows)]
fn is_windows_reparse_point(metadata: &std::fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt as _;

    metadata.file_attributes() & 0x400 != 0
}

#[cfg(not(windows))]
const fn is_windows_reparse_point(_metadata: &std::fs::Metadata) -> bool {
    false
}
