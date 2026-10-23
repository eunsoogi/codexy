//! Exercises the shared cancellation-capable MCP stdio loop without a Watcher server.

use super::*;
use anyhow::anyhow;
use codexy_runtime::mcp::{ToolDef, run_stdio_server_with_cancellation};
use std::error::Error;
use std::io::{BufRead as _, BufReader};
use std::sync::mpsc::{self, Receiver};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

const SERVER_MODE_ENV: &str = "CODEXY_MCP_CANCELLATION_TEST_SERVER";
const CHILD_TEST_FILTER: &str = "system::mcp_stdio::cancellation::shared_cancellation_preserves_request_lifecycle";
const RESPONSE_TIMEOUT: Duration = Duration::from_secs(5);

#[test]
fn shared_cancellation_preserves_request_lifecycle() -> Result<(), Box<dyn Error>> {
    // Re-enter this test in a child process to drive the public stdio server over real pipes.
    if std::env::var_os(SERVER_MODE_ENV).is_some() {
        run_test_server()?;
        return Ok(());
    }

    let mut command = Command::new(std::env::current_exe()?);
    command.args(["--exact", CHILD_TEST_FILTER, "--nocapture"]);
    command.env(SERVER_MODE_ENV, "1");
    let mut client = McpClient::spawn_command(command)?;
    let (responses, reader) = spawn_response_reader(&mut client)?;
    let marker_dir = tempfile::tempdir()?;
    let started_marker = marker_dir.path().join("blocked-tool-started");

    client.send_without_read(&json!({
        "jsonrpc":"2.0","id":7,"method":"tools/call",
        "params":{"name":"blocked","arguments":{"startedMarker":started_marker.display().to_string()}}
    }))?;
    let deadline = Instant::now() + RESPONSE_TIMEOUT;
    while !started_marker.exists() && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(1));
    }
    assert!(started_marker.exists(), "blocked tool did not start in the child server");

    client.send_without_read(&json!({
        "jsonrpc":"2.0","id":7,"method":"tools/call",
        "params":{"name":"fast","arguments":{}}
    }))?;
    let duplicate = receive_response(&responses)?;
    assert_eq!(duplicate["id"], 7);
    assert_eq!(duplicate["error"]["code"], -32600);

    client.send_without_read(&json!({
        "jsonrpc":"2.0","method":"notifications/cancelled",
        "params":{"requestId":7,"reason":"test cancellation"}
    }))?;
    client.send_without_read(&json!({
        "jsonrpc":"2.0","id":7,"method":"tools/call",
        "params":{"name":"fast","arguments":{}}
    }))?;
    let reused = receive_response(&responses)?;
    assert_eq!(reused["id"], 7);
    assert_eq!(
        reused["result"]["tool"], "fast",
        "the cancelled call must not emit a late response before the reused request id"
    );

    drop(client.child.stdin.take());
    let status = client.child.wait()?;
    assert!(status.success(), "child MCP server exited with {status}");
    reader
        .join()
        .map_err(|_| std::io::Error::other("MCP response reader thread panicked"))?;
    Ok(())
}

fn run_test_server() -> Result<(), Box<dyn Error>> {
    let tools = [
        ToolDef::new("blocked", "Waits for cancellation in this test fixture.", json!({"type":"object"})),
        ToolDef::new("fast", "Returns immediately in this test fixture.", json!({"type":"object"})),
    ];
    run_stdio_server_with_cancellation("codexy-test", "1", &tools, |name, arguments, token| {
        if name == "blocked" {
            let marker = arguments["startedMarker"]
                .as_str()
                .ok_or_else(|| anyhow!("blocked tool requires a startup marker path"))?;
            std::fs::write(marker, b"started")?;
            while !token.is_cancelled() {
                thread::sleep(Duration::from_millis(1));
            }
            // Keep the call in flight briefly so cancellation precedes its normal result.
            thread::sleep(Duration::from_millis(20));
        }
        Ok(json!({"tool": name}))
    })?;
    Ok(())
}

fn spawn_response_reader(
    client: &mut McpClient,
) -> Result<(Receiver<Result<Value, String>>, JoinHandle<()>), Box<dyn Error>> {
    let stdout = client.child.stdout.take().ok_or("missing child stdout")?;
    let (sender, receiver) = mpsc::channel();
    let reader = thread::spawn(move || {
        let mut stdout = BufReader::new(stdout);
        loop {
            let mut line = String::new();
            match stdout.read_line(&mut line) {
                Ok(0) => {
                    let _ = sender.send(Err("child MCP server closed stdout early".to_owned()));
                    return;
                }
                Ok(_) => {}
                Err(error) => {
                    let _ = sender.send(Err(format!("reading child MCP stdout: {error}")));
                    return;
                }
            }
            let line = line.trim();
            if line.is_empty() || !line.starts_with('{') {
                continue;
            }
            match serde_json::from_str(line) {
                Ok(response) => {
                    if sender.send(Ok(response)).is_err() {
                        return;
                    }
                }
                Err(error) => {
                    let _ = sender.send(Err(format!("invalid child MCP response: {error}")));
                    return;
                }
            }
        }
    });
    Ok((receiver, reader))
}

fn receive_response(
    receiver: &Receiver<Result<Value, String>>,
) -> Result<Value, Box<dyn Error>> {
    match receiver.recv_timeout(RESPONSE_TIMEOUT) {
        Ok(Ok(response)) => Ok(response),
        Ok(Err(error)) => Err(std::io::Error::other(error).into()),
        Err(error) => Err(std::io::Error::new(std::io::ErrorKind::TimedOut, error).into()),
    }
}
