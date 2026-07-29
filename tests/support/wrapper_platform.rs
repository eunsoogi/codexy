use std::path::Path;

use super::make_executable;

/// A host selected only inside a copied wrapper fixture. Production wrappers continue to
/// source their real-host detector.
#[derive(Clone, Copy, Debug)]
pub(crate) enum FixturePlatform {
    DarwinArm64,
    LinuxX86_64,
    WindowsX86_64,
    Unsupported,
}

impl FixturePlatform {
    fn runtime_name(self) -> &'static str {
        match self {
            Self::DarwinArm64 => "darwin-arm64",
            Self::LinuxX86_64 => "linux-x86_64",
            Self::WindowsX86_64 => "windows-x86_64",
            Self::Unsupported => "unknown-unknown",
        }
    }
}

pub(crate) fn install_fixture_platform(
    plugin_root: &Path,
    platform: FixturePlatform,
) -> std::io::Result<()> {
    let selector = plugin_root.join("mcp/runtime-platform.sh");
    std::fs::write(
        &selector,
        format!(
            "#!/bin/sh\ncodexy_runtime_platform() {{\n  printf '%s\\n' '{}'\n}}\n",
            platform.runtime_name()
        ),
    )?;
    make_executable(&selector)
}

#[cfg(test)]
mod controls {
    use super::*;

    #[test]
    fn fixture_platform_selector_is_explicit_and_never_reads_host_environment()
    -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let mcp = temp.path().join("mcp");
        std::fs::create_dir(&mcp)?;
        install_fixture_platform(temp.path(), FixturePlatform::WindowsX86_64)?;
        assert_eq!(
            std::fs::read_to_string(mcp.join("runtime-platform.sh"))?,
            "#!/bin/sh\ncodexy_runtime_platform() {\n  printf '%s\\n' 'windows-x86_64'\n}\n"
        );
        Ok(())
    }

    #[test]
    fn unsupported_fixture_platform_fails_closed() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        std::fs::create_dir(temp.path().join("mcp"))?;
        install_fixture_platform(temp.path(), FixturePlatform::Unsupported)?;
        assert_eq!(
            std::fs::read_to_string(temp.path().join("mcp/runtime-platform.sh"))?,
            "#!/bin/sh\ncodexy_runtime_platform() {\n  printf '%s\\n' 'unknown-unknown'\n}\n"
        );
        Ok(())
    }
}
