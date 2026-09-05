use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread::JoinHandle;

use anyhow::{Context as _, Result};
use serde_json::Value;

use crate::lsp::protocol::LspRequest;
use crate::lsp::session_io::{SharedStderr, spawn_stderr_reader, spawn_stdout_reader};

#[derive(Debug)]
pub(super) struct LspSession {
    pub(super) child: Child,
    pub(super) stdin: ChildStdin,
    pub(super) rx: Receiver<Value>,
    pub(super) stderr: SharedStderr,
    pub(super) stdout_reader: Option<JoinHandle<()>>,
    pub(super) stderr_reader: Option<JoinHandle<()>>,
    pub(super) next_id: i64,
    pub(super) notifications: Vec<Value>,
    pub(super) opened_documents: std::collections::BTreeMap<String, i32>,
}

impl LspSession {
    pub(super) fn spawn(request: &LspRequest) -> Result<Self> {
        let command = request
            .server
            .command
            .as_ref()
            .filter(|items| !items.is_empty())
            .context("server command is missing")?;
        let executable = command.first().context("server command is missing")?;
        let workspace_root = request.workspace_root_path();
        let mut child = Command::new(executable)
            .args(command.iter().skip(1))
            .current_dir(&workspace_root)
            .envs(std::env::vars_os())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .with_context(|| format!("spawn LSP server {}", request.server.id))?;
        let stdout = child
            .stdout
            .take()
            .context("LSP server stdout unavailable")?;
        let stderr = child
            .stderr
            .take()
            .context("LSP server stderr unavailable")?;
        let stdin = child.stdin.take().context("LSP server stdin unavailable")?;
        let (tx, rx) = mpsc::channel();
        let stderr_buffer = SharedStderr::default();
        let stdout_reader = spawn_stdout_reader(stdout, tx, &stderr_buffer);
        let stderr_reader = spawn_stderr_reader(stderr, &stderr_buffer);
        Ok(Self {
            child,
            stdin,
            rx,
            stderr: stderr_buffer,
            stdout_reader: Some(stdout_reader),
            stderr_reader: Some(stderr_reader),
            next_id: 1,
            notifications: Vec::new(),
            opened_documents: std::collections::BTreeMap::new(),
        })
    }
}
