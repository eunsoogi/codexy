use std::path::Path;

pub trait FixtureHost {
    fn manifest_dir(&self) -> &Path;
    fn validate_config_binary(&self) -> &Path;
    fn mcp_lsp_binary(&self) -> &Path;
    fn mcp_codegraph_binary(&self) -> &Path;
    fn make_executable(&self, path: &Path) -> std::io::Result<()>;
    fn materialize_admission_runtime_suite(&self, plugin_root: &Path) -> std::io::Result<()>;
}
