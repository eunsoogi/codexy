use std::collections::HashMap;
use std::io::{self, Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread::{self, JoinHandle};

use anyhow::{Context as _, Result, anyhow};
use serde_json::{Value, json};

use super::{FrameParser, ToolDef, error_response, handle_control_message};

const MAX_IN_FLIGHT_REQUESTS: usize = 16;

/// Cooperative cancellation state for one in-flight MCP request.
#[derive(Clone, Debug)]
pub struct CancellationToken(Arc<AtomicBool>);

impl CancellationToken {
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }

    fn new() -> Self {
        Self(Arc::new(AtomicBool::new(false)))
    }

    fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }
}

/// Runs an MCP server whose tool calls can be cancelled by a subsequent
/// `notifications/cancelled` message on the same stdio connection.
pub fn run_stdio_server_with_cancellation<F>(
    name: &str,
    version: &str,
    tools: &[ToolDef],
    call_tool: F,
) -> Result<()>
where
    F: Fn(&str, &Value, &CancellationToken) -> Result<Value> + Send + Sync + 'static,
{
    let mut parser = FrameParser::default();
    let mut chunk = [0_u8; 8192];
    let calls = Arc::new(Mutex::new(HashMap::<String, CancellationToken>::new()));
    let output = Arc::new(Mutex::new(io::stdout()));
    let call_tool = Arc::new(call_tool);
    let mut workers = Vec::new();
    loop {
        let read = {
            let stdin = io::stdin();
            let mut stdin = stdin.lock();
            stdin.read(&mut chunk).context("reading MCP stdin")?
        };
        if read == 0 {
            cancel_all(&calls);
            join_workers(workers);
            return Ok(());
        }
        parser.extend(&chunk[..read])?;
        workers.retain(|worker| !worker.is_finished());
        while let Some(message) = parser.next_frame()? {
            dispatch_message(
                name,
                version,
                tools,
                &message,
                &calls,
                &output,
                &mut workers,
                &call_tool,
            )?;
            workers.retain(|worker| !worker.is_finished());
        }
    }
}

type CancellationMap = Arc<Mutex<HashMap<String, CancellationToken>>>;
type SharedOutput = Arc<Mutex<io::Stdout>>;

#[allow(clippy::too_many_arguments)]
fn dispatch_message<F>(
    name: &str,
    version: &str,
    tools: &[ToolDef],
    message: &Value,
    calls: &CancellationMap,
    output: &SharedOutput,
    workers: &mut Vec<JoinHandle<()>>,
    call_tool: &Arc<F>,
) -> Result<()>
where
    F: Fn(&str, &Value, &CancellationToken) -> Result<Value> + Send + Sync + 'static,
{
    let method = message
        .get("method")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if method == "notifications/cancelled" {
        if let Some(request_key) = cancellation_key(message) {
            if let Ok(active) = calls.lock() {
                if let Some(token) = active.get(&request_key) {
                    token.cancel();
                }
            }
        }
        return Ok(());
    }
    if method != "tools/call" {
        if let Some(response) = handle_control_message(name, version, tools, message) {
            write_shared_frame(output, &response)?;
        }
        return Ok(());
    }
    let Some(id) = message.get("id").cloned() else {
        return Ok(());
    };
    let request_key = serde_json::to_string(&id).context("serializing MCP request id")?;
    let token = CancellationToken::new();
    {
        let mut active = lock(calls)?;
        if active.contains_key(&request_key) {
            write_shared_frame(
                output,
                &error_response(&id, -32600, "MCP request id is already in flight"),
            )?;
            return Ok(());
        }
        if active.len() >= MAX_IN_FLIGHT_REQUESTS {
            write_shared_frame(
                output,
                &error_response(&id, -32000, "too many in-flight MCP requests"),
            )?;
            return Ok(());
        }
        active.insert(request_key.clone(), token.clone());
    }
    let params = message.get("params").cloned().unwrap_or(Value::Null);
    let tool_name = params
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();
    let arguments = params.get("arguments").cloned().unwrap_or(Value::Null);
    let calls_for_worker = Arc::clone(calls);
    let output_for_worker = Arc::clone(output);
    let call_tool = Arc::clone(call_tool);
    let worker = thread::Builder::new()
        .name(format!("codexy-mcp-{tool_name}"))
        .spawn(move || {
            let result = call_tool(&tool_name, &arguments, &token);
            if !token.is_cancelled() {
                let response = match result {
                    Ok(result) => json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": result
                    }),
                    Err(error) => error_response(&id, -32000, &error.to_string()),
                };
                let _ = write_shared_frame(&output_for_worker, &response);
            }
            finish_request(&calls_for_worker, &request_key, &token);
        })
        .context("starting MCP tool worker")?;
    workers.push(worker);
    Ok(())
}

fn cancellation_key(message: &Value) -> Option<String> {
    let request_id = message.get("params")?.get("requestId")?;
    if !matches!(request_id, Value::String(_) | Value::Number(_)) {
        return None;
    }
    serde_json::to_string(request_id).ok()
}

fn lock<T>(mutex: &Mutex<T>) -> Result<MutexGuard<'_, T>> {
    mutex
        .lock()
        .map_err(|_| anyhow!("MCP server state lock is poisoned"))
}

fn write_shared_frame(output: &SharedOutput, payload: &Value) -> Result<()> {
    let mut output = lock(output)?;
    output.write_all(&serde_json::to_vec(payload)?)?;
    output.write_all(b"\n")?;
    output.flush()?;
    Ok(())
}

fn finish_request(calls: &CancellationMap, request_key: &str, token: &CancellationToken) {
    if let Ok(mut active) = calls.lock() {
        if active
            .get(request_key)
            .is_some_and(|current| Arc::ptr_eq(&current.0, &token.0))
        {
            active.remove(request_key);
        }
    }
}

fn cancel_all(calls: &CancellationMap) {
    if let Ok(active) = calls.lock() {
        for token in active.values() {
            token.cancel();
        }
    }
}

fn join_workers(workers: Vec<JoinHandle<()>>) {
    for worker in workers {
        let _ = worker.join();
    }
}
