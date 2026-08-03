use crate::support::FixtureCommand as Command;
use std::time::Duration;

use crate::support::{WrapperFixture, run_wrapper_command_with_timeout};

#[test]
fn mcp_wrappers_order_runtime_dir_then_bundled_then_pinned_uvx()
-> Result<(), Box<dyn std::error::Error>> {
    for server in ["lsp", "codegraph"] {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(format!("plugins/codexy/mcp/codexy-mcp-{server}"));
        let wrapper = std::fs::read_to_string(&path)?;
        let override_index = required(&wrapper, "CODEXY_RUNTIME_DIR", &path)?;
        let bundled_index = required(&wrapper, "if [ -x \"$bundled_runtime\" ]; then", &path)?;
        let uvx_index = required(&wrapper, "exec uvx --from getcodexy==1.2.2", &path)?;
        assert!(override_index < bundled_index && bundled_index < uvx_index);
    }
    Ok(())
}

#[test]
fn wrapper_subprocess_timeout_is_actionable() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let fixture = WrapperFixture::new(temp.path())?;
    fixture.replace_wrapper("lsp", "#!/bin/sh\nexec sleep 45\n")?;
    let mut command = Command::new(fixture.plugin_root.join("mcp/codexy-mcp-lsp"));
    let error = run_wrapper_command_with_timeout(&mut command, Duration::from_secs(2))
        .expect_err("wrapper subprocess must time out");
    assert!(error.to_string().contains("timed out"));
    Ok(())
}

fn required(
    text: &str,
    needle: &str,
    path: &std::path::Path,
) -> Result<usize, Box<dyn std::error::Error>> {
    text.find(needle)
        .ok_or_else(|| format!("{} missing {needle}", path.display()).into())
}
