use std::path::Path;

use anyhow::{Context as _, Result, bail};

use crate::paths::display_relative;

#[path = "runtime_binary/windows_delegates.rs"]
mod windows_delegates;

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

pub(super) fn check_windows_dispatcher(plugin_root: &Path) -> Result<()> {
    let entrypoint = plugin_root.join("mcp/codexy-mcp-devtools.exe");
    if !entrypoint.is_file() {
        bail!(
            "{} native Windows MCP dispatcher missing",
            display_relative(&entrypoint)
        );
    }
    reject_link_or_reparse_point(plugin_root, &entrypoint)?;
    let launcher = std::fs::read(&entrypoint)?;
    if !is_x86_64_pe(&launcher) {
        bail!(
            "{} native Windows MCP dispatcher has invalid binary format",
            display_relative(&entrypoint),
        );
    }
    for server in ["lsp", "codegraph"] {
        let legacy = plugin_root
            .join("mcp")
            .join(format!("codexy-mcp-{server}.exe"));
        if legacy.exists() {
            bail!(
                "{} duplicate native Windows MCP entrypoint must not be packaged",
                display_relative(&legacy)
            );
        }
        let runtime = plugin_root
            .join("runtime")
            .join(artifact_name(server, "windows-x86_64"));
        if launcher == std::fs::read(&runtime)? {
            bail!(
                "{} native Windows MCP dispatcher must not duplicate {}",
                display_relative(&entrypoint),
                display_relative(&runtime)
            );
        }
        windows_delegates::check(plugin_root, server)?;
    }
    Ok(())
}

pub(super) fn check_distinct_server_runtimes(plugin_root: &Path, platform: &str) -> Result<()> {
    let lsp = plugin_root
        .join("runtime")
        .join(artifact_name("lsp", platform));
    let codegraph = plugin_root
        .join("runtime")
        .join(artifact_name("codegraph", platform));
    if std::fs::read(&lsp)? == std::fs::read(&codegraph)? {
        bail!(
            "{} and {} must not contain duplicate native MCP runtime bytes",
            display_relative(&lsp),
            display_relative(&codegraph)
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

#[cfg(test)]
mod tests {
    use super::*;

    const PACKAGED_RUNTIME_CASES: [(&str, &str); 6] = [
        ("codexy-mcp-lsp-darwin-arm64.bin", "darwin-arm64"),
        ("codexy-mcp-codegraph-darwin-arm64.bin", "darwin-arm64"),
        ("codexy-mcp-lsp-linux-x86_64.bin", "linux-x86_64"),
        ("codexy-mcp-codegraph-linux-x86_64.bin", "linux-x86_64"),
        ("codexy-mcp-lsp-windows-x86_64.exe", "windows-x86_64"),
        ("codexy-mcp-codegraph-windows-x86_64.exe", "windows-x86_64"),
    ];

    #[test]
    fn invalid_runtime_format_matrix_preserves_cli_inputs_and_diagnostics()
    -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        for (runtime_name, platform) in PACKAGED_RUNTIME_CASES {
            let path = temp.path().join(runtime_name);
            std::fs::write(&path, b"#!/bin/sh\nexit 0\n")?;
            assert_invalid_format(&path, platform)?;
        }

        let invalid_signature = temp.path().join("invalid-signature.exe");
        let mut bytes = valid_runtime_bytes("windows-x86_64");
        bytes[0x80..0x84].copy_from_slice(b"PX\0\0");
        std::fs::write(&invalid_signature, bytes)?;
        assert_invalid_format(&invalid_signature, "windows-x86_64")?;

        for scenario in ["x86", "pe32", "dll", "missing-optional-header", "truncated"] {
            let path = temp.path().join(format!("{scenario}.exe"));
            let mut bytes = valid_runtime_bytes("windows-x86_64");
            match scenario {
                "x86" => bytes[0x84..0x86].copy_from_slice(&0x014c_u16.to_le_bytes()),
                "pe32" => bytes[0x98..0x9a].copy_from_slice(&0x010b_u16.to_le_bytes()),
                "dll" => bytes[0x96..0x98].copy_from_slice(&0x2022_u16.to_le_bytes()),
                "missing-optional-header" => {
                    bytes[0x94..0x96].copy_from_slice(&0_u16.to_le_bytes());
                }
                "truncated" => bytes.truncate(0x90),
                _ => unreachable!(),
            }
            std::fs::write(&path, bytes)?;
            assert_invalid_format(&path, "windows-x86_64")?;
        }
        Ok(())
    }

    fn assert_invalid_format(
        path: &Path,
        platform: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let error = check(path, platform).expect_err("invalid runtime format unexpectedly passed");
        assert!(
            error.to_string().ends_with(&format!(
                "bundled MCP runtime has invalid binary format for {platform}"
            )),
            "unexpected runtime format diagnostic: {error}"
        );
        Ok(())
    }

    fn valid_runtime_bytes(platform: &str) -> Vec<u8> {
        let mut bytes = if platform == "windows-x86_64" {
            let mut bytes = vec![0; 4096];
            bytes[0..2].copy_from_slice(b"MZ");
            bytes[0x3c..0x40].copy_from_slice(&(0x80_u32).to_le_bytes());
            bytes[0x80..0x84].copy_from_slice(b"PE\0\0");
            bytes[0x84..0x86].copy_from_slice(&0x8664_u16.to_le_bytes());
            bytes[0x86..0x88].copy_from_slice(&1_u16.to_le_bytes());
            bytes[0x94..0x96].copy_from_slice(&0xf0_u16.to_le_bytes());
            bytes[0x96..0x98].copy_from_slice(&0x0022_u16.to_le_bytes());
            bytes[0x98..0x9a].copy_from_slice(&0x20b_u16.to_le_bytes());
            bytes
        } else if platform == "darwin-arm64" {
            vec![0xcf, 0xfa, 0xed, 0xfe]
        } else {
            vec![0x7f, b'E', b'L', b'F']
        };
        bytes.resize(4096, 0);
        bytes
    }
}
