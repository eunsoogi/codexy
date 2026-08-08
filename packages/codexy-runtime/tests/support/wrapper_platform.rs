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
