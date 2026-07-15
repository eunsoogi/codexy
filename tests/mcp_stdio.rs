use std::io::{Read as _, Write as _};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use serde_json::{Value, json};

#[path = "mcp_stdio/client.rs"]
mod client;
#[path = "mcp_stdio/codegraph_protocol.rs"]
mod codegraph_protocol;
#[path = "mcp_stdio/fixtures.rs"]
mod fixtures;
#[path = "mcp_stdio/lsp_protocol.rs"]
mod lsp_protocol;
#[path = "mcp_stdio/wrapper_runtime.rs"]
mod wrapper_runtime;

use client::{InstalledPlugin, McpClient, TempRuntimeDir};
use fixtures::{installed_plugin_copy, installed_plugin_under_rust_host, temp_runtime_dir};
