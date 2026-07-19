use std::io::{Read as _, Write as _};
use std::process::{Child, Command, Stdio};

use serde_json::{Value, json};

#[path = "mcp_stdio/client.rs"]
mod client;
#[path = "mcp_stdio/codegraph_protocol.rs"]
mod codegraph_protocol;
#[path = "mcp_stdio/lsp_protocol.rs"]
mod lsp_protocol;

use client::McpClient;
