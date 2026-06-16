use std::io::{Read as _, Write as _};
use std::path::Path;
use std::process::{Child, Command, Stdio};

use serde_json::{Value, json};

struct McpClient {
    child: Child,
    buffer: Vec<u8>,
}

impl McpClient {
    fn spawn(binary: &str) -> Result<Self, Box<dyn std::error::Error>> {
        Self::spawn_with(binary, None)
    }

    fn spawn_in(binary: &str, cwd: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        Self::spawn_with(binary, Some(cwd))
    }

    fn spawn_with(binary: &str, cwd: Option<&Path>) -> Result<Self, Box<dyn std::error::Error>> {
        let mut command = Command::new(binary);
        if let Some(cwd) = cwd {
            command.current_dir(cwd);
        }
        Self::spawn_command(command)
    }

    fn spawn_command(mut command: Command) -> Result<Self, Box<dyn std::error::Error>> {
        let child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        Ok(Self {
            child,
            buffer: Vec::new(),
        })
    }

    fn send(&mut self, payload: &Value) -> Result<Value, Box<dyn std::error::Error>> {
        let body = serde_json::to_vec(&payload)?;
        let stdin = self.child.stdin.as_mut().ok_or("missing child stdin")?;
        write!(stdin, "Content-Length: {}\r\n\r\n", body.len())?;
        stdin.write_all(&body)?;
        stdin.flush()?;
        self.read_frame()
    }

    fn read_frame(&mut self) -> Result<Value, Box<dyn std::error::Error>> {
        loop {
            if let Some(header_end) = self
                .buffer
                .windows(4)
                .position(|window| window == b"\r\n\r\n")
            {
                let header = std::str::from_utf8(&self.buffer[..header_end])?;
                let length = header
                    .lines()
                    .find_map(|line| {
                        let (name, value) = line.split_once(':')?;
                        name.eq_ignore_ascii_case("content-length")
                            .then(|| value.trim().parse::<usize>().ok())
                            .flatten()
                    })
                    .ok_or("missing Content-Length")?;
                let body_start = header_end + 4;
                let body_end = body_start + length;
                if self.buffer.len() >= body_end {
                    let body = self.buffer[body_start..body_end].to_vec();
                    self.buffer.drain(..body_end);
                    return Ok(serde_json::from_slice(&body)?);
                }
            }
            let mut chunk = [0_u8; 4096];
            let stdout = self.child.stdout.as_mut().ok_or("missing child stdout")?;
            let read = stdout.read(&mut chunk)?;
            if read == 0 {
                let mut stderr = String::new();
                if let Some(output) = self.child.stderr.as_mut() {
                    output.read_to_string(&mut stderr)?;
                }
                return Err(format!("MCP process exited before frame: {stderr}").into());
            }
            self.buffer.extend_from_slice(&chunk[..read]);
        }
    }
}

#[test]
fn lsp_wrapper_uses_installed_plugin_root_for_config() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let installed_plugin = temp.path().join("codexy");
    copy_dir(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("plugins/codexy")
            .as_path(),
        &installed_plugin,
    )?;
    let lsp_config_path = installed_plugin.join(".codex/lsp-client.json");
    let mut lsp_config: Value = serde_json::from_str(&std::fs::read_to_string(&lsp_config_path)?)?;
    lsp_config["lsp"]["codexy-installed-root"] = json!({
        "extensions": [".installed"],
        "priority": 999,
        "command": [env!("CARGO_BIN_EXE_codexy-fake-lsp")]
    });
    std::fs::write(&lsp_config_path, serde_json::to_vec_pretty(&lsp_config)?)?;

    let mut command = Command::new(installed_plugin.join("bin/codexy-mcp-lsp"));
    command
        .current_dir(&installed_plugin)
        .env(
            "PATH",
            format!(
                "{}:{}",
                std::path::Path::new(env!("CARGO_BIN_EXE_codexy-mcp-lsp"))
                    .parent()
                    .ok_or("runtime bin dir")?
                    .display(),
                std::env::var("PATH")?
            ),
        )
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut client = McpClient::spawn_command(command)?;
    let _init = client.send(&json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}))?;
    let response = client.send(&json!({
        "jsonrpc":"2.0","id":2,"method":"tools/call",
        "params":{"name":"lsp_for_path","arguments":{"path":"sample.installed"}}
    }))?;
    let payload: Value = serde_json::from_str(
        response["result"]["content"][0]["text"]
            .as_str()
            .ok_or("lsp_for_path text")?,
    )?;
    assert!(
        payload
            .as_array()
            .ok_or("lsp_for_path payload must be array")?
            .iter()
            .any(|server| server["id"] == "codexy-installed-root"),
        "wrapper-launched runtime must read LSP config from copied installed plugin, got {payload:#}"
    );
    Ok(())
}

impl Drop for McpClient {
    fn drop(&mut self) {
        drop(self.child.stdin.take());
        let _ = self.child.wait();
    }
}

fn copy_dir(source: &Path, target: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(target)?;
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());
        if source_path.is_dir() {
            copy_dir(&source_path, &target_path)?;
        } else {
            std::fs::copy(source_path, target_path)?;
        }
    }
    Ok(())
}

#[test]
fn codegraph_stdio_indexes_searches_and_bounds_missing_neighbors()
-> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    std::fs::write(root.path().join("dep.rs"), "pub const VALUE: u8 = 1;\n")?;
    std::fs::write(
        root.path().join("entry.rs"),
        "mod dep;\npub const ENTRY: u8 = dep::VALUE;\n",
    )?;
    std::fs::write(
        root.path().join("extra_one.rs"),
        "pub const ENTRY_ONE: u8 = 1;\n",
    )?;
    std::fs::write(
        root.path().join("extra_two.rs"),
        "pub const ENTRY_TWO: u8 = 2;\n",
    )?;

    let mut client = McpClient::spawn(env!("CARGO_BIN_EXE_codexy-mcp-codegraph"))?;
    let init = client.send(&json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}))?;
    assert_eq!(init["result"]["serverInfo"]["name"], "codexy-codegraph");
    let list = client.send(&json!({"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}))?;
    assert!(
        list["result"]["tools"]
            .as_array()
            .ok_or("tools must be array")?
            .iter()
            .any(|tool| tool["name"] == "codegraph_index")
    );
    let index = client.send(&json!({
        "jsonrpc":"2.0","id":3,"method":"tools/call",
        "params":{"name":"codegraph_index","arguments":{"root":root.path(),"limit":10}}
    }))?;
    let graph: Value = serde_json::from_str(
        index["result"]["content"][0]["text"]
            .as_str()
            .ok_or("text")?,
    )?;
    assert!(
        graph["edges"]
            .as_array()
            .ok_or("edges must be array")?
            .iter()
            .any(|edge| edge["from"] == "entry.rs" && edge["to"] == "dep.rs")
    );
    let search = client.send(&json!({
        "jsonrpc":"2.0","id":4,"method":"tools/call",
        "params":{"name":"codegraph_search","arguments":{"root":root.path(),"query":"ENTRY","limit":1.0}}
    }))?;
    let search_text = search["result"]["content"][0]["text"]
        .as_str()
        .ok_or("search text")?;
    assert!(
        search_text.contains("ENTRY"),
        "codegraph_search must return a matching line, got {search_text:?}"
    );
    assert_eq!(
        search_text.lines().count(),
        1,
        "codegraph_search must stop at the requested line limit"
    );
    let missing = client.send(&json!({
        "jsonrpc":"2.0","id":5,"method":"tools/call",
        "params":{"name":"codegraph_neighbors","arguments":{"root":root.path(),"path":"missing.rs"}}
    }))?;
    let neighbors: Value = serde_json::from_str(
        missing["result"]["content"][0]["text"]
            .as_str()
            .ok_or("text")?,
    )?;
    assert_eq!(neighbors, json!([]));
    Ok(())
}

#[test]
fn codegraph_stdio_matches_absolute_paths_when_root_is_relative()
-> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let dependency = root.path().join("dep.rs");
    let entry = root.path().join("entry.rs");
    std::fs::write(&dependency, "pub const VALUE: u8 = 1;\n")?;
    std::fs::write(&entry, "mod dep;\npub const ENTRY: u8 = dep::VALUE;\n")?;

    let mut client = McpClient::spawn_in(env!("CARGO_BIN_EXE_codexy-mcp-codegraph"), root.path())?;
    let _init = client.send(&json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}))?;
    let reverse_deps = client.send(&json!({
        "jsonrpc":"2.0","id":2,"method":"tools/call",
        "params":{"name":"codegraph_reverse_deps","arguments":{"root":".","path":dependency,"limit":10}}
    }))?;
    let reverse_payload: Value = serde_json::from_str(
        reverse_deps["result"]["content"][0]["text"]
            .as_str()
            .ok_or("reverse deps text")?,
    )?;
    assert!(
        reverse_payload["dependents"]
            .as_array()
            .ok_or("reverse dependents must be array")?
            .iter()
            .any(|dependent| dependent["path"] == "entry.rs"),
        "absolute dependency path should match relative graph edges"
    );

    let neighborhood = client.send(&json!({
        "jsonrpc":"2.0","id":3,"method":"tools/call",
        "params":{"name":"codegraph_neighborhood","arguments":{"root":".","path":entry,"depth":0.0,"limit":10.0}}
    }))?;
    let neighborhood_payload: Value = serde_json::from_str(
        neighborhood["result"]["content"][0]["text"]
            .as_str()
            .ok_or("neighborhood text")?,
    )?;
    let nodes = neighborhood_payload["nodes"]
        .as_array()
        .ok_or("neighborhood nodes must be array")?;
    assert!(nodes.iter().any(|node| node["path"] == "entry.rs"));
    assert!(
        !nodes.iter().any(|node| node["path"] == "dep.rs"),
        "float-encoded depth must be honored"
    );
    Ok(())
}

#[test]
fn codegraph_stdio_keeps_outside_absolute_paths_distinct() -> Result<(), Box<dyn std::error::Error>>
{
    let root = tempfile::tempdir()?;
    let outside = tempfile::tempdir()?;
    let outside_dep = outside.path().join("dep.rs");
    std::fs::write(&outside_dep, "pub const OUTSIDE: u8 = 1;\n")?;
    let canonical_outside = outside_dep.canonicalize()?;
    let mirrored_dep = root.path().join(canonical_outside.strip_prefix("/")?);
    let mirrored_dir = mirrored_dep.parent().ok_or("mirrored parent")?;
    std::fs::create_dir_all(mirrored_dir)?;
    std::fs::write(
        &mirrored_dep,
        "mod leaf;\npub const MIRRORED: u8 = leaf::LEAF;\n",
    )?;
    std::fs::write(mirrored_dir.join("leaf.rs"), "pub const LEAF: u8 = 1;\n")?;
    std::fs::write(
        mirrored_dir.join("entry.rs"),
        "mod dep;\npub const ENTRY: u8 = dep::MIRRORED;\n",
    )?;

    let mut client = McpClient::spawn(env!("CARGO_BIN_EXE_codexy-mcp-codegraph"))?;
    let _init = client.send(&json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}))?;
    let reverse_deps = client.send(&json!({
        "jsonrpc":"2.0","id":2,"method":"tools/call",
        "params":{"name":"codegraph_reverse_deps","arguments":{"root":root.path(),"path":outside_dep,"limit":10}}
    }))?;
    let reverse_payload: Value = serde_json::from_str(
        reverse_deps["result"]["content"][0]["text"]
            .as_str()
            .ok_or("reverse deps text")?,
    )?;
    assert!(
        reverse_payload["dependents"]
            .as_array()
            .ok_or("reverse dependents must be array")?
            .is_empty(),
        "outside absolute path must not alias mirrored in-root reverse deps"
    );

    let neighborhood = client.send(&json!({
        "jsonrpc":"2.0","id":3,"method":"tools/call",
        "params":{"name":"codegraph_neighborhood","arguments":{"root":root.path(),"path":outside_dep,"depth":1,"limit":10}}
    }))?;
    let neighborhood_payload: Value = serde_json::from_str(
        neighborhood["result"]["content"][0]["text"]
            .as_str()
            .ok_or("neighborhood text")?,
    )?;
    assert!(
        neighborhood_payload["edges"]
            .as_array()
            .ok_or("neighborhood edges must be array")?
            .is_empty(),
        "outside absolute path must not alias mirrored in-root neighborhood edges"
    );
    let nodes = neighborhood_payload["nodes"]
        .as_array()
        .ok_or("neighborhood nodes must be array")?;
    assert!(
        !nodes.iter().any(|node| {
            node["path"]
                .as_str()
                .is_some_and(|path| path.ends_with("leaf.rs"))
        }),
        "outside absolute path must not traverse mirrored in-root imports"
    );
    Ok(())
}

#[test]
fn lsp_stdio_reports_status_diagnostics_and_unmatched_extensions()
-> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let source = root.path().join("sample.toml");
    std::fs::write(&source, "value = 1\n")?;
    let fake_lsp = env!("CARGO_BIN_EXE_codexy-fake-lsp");

    let mut client = Command::new(env!("CARGO_BIN_EXE_codexy-mcp-lsp"))
        .env("CODEXY_LSP_ALLOW_COMMAND_OVERRIDE", "1")
        .env("CODEXY_FAKE_LSP_PULL_DIAGNOSTICS", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map(|child| McpClient {
            child,
            buffer: Vec::new(),
        })?;
    let init = client.send(&json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}))?;
    assert_eq!(init["result"]["serverInfo"]["name"], "codexy-lsp");
    let server = json!({"id":"taplo","command":[fake_lsp]});
    let status = client.send(&json!({
        "jsonrpc":"2.0","id":2,"method":"tools/call",
        "params":{"name":"lsp_status","arguments":{"root":root.path(),"path":"sample.toml","server":server}}
    }))?;
    let status_payload: Value = serde_json::from_str(
        status["result"]["content"][0]["text"]
            .as_str()
            .ok_or("text")?,
    )?;
    assert_eq!(status_payload["available"], true);
    let diagnostics = client.send(&json!({
        "jsonrpc":"2.0","id":3,"method":"tools/call",
        "params":{"name":"lsp_diagnostics","arguments":{"root":root.path(),"path":"sample.toml","server":server,"timeoutMs":5000}}
    }))?;
    let diagnostics_payload: Value = serde_json::from_str(
        diagnostics["result"]["content"][0]["text"]
            .as_str()
            .ok_or("text")?,
    )?;
    assert_eq!(diagnostics_payload["status"], "ok");
    let unmatched = client.send(&json!({
        "jsonrpc":"2.0","id":4,"method":"tools/call",
        "params":{"name":"lsp_status","arguments":{"root":root.path(),"path":"sample.unknown"}}
    }))?;
    let unmatched_payload: Value = serde_json::from_str(
        unmatched["result"]["content"][0]["text"]
            .as_str()
            .ok_or("text")?,
    )?;
    assert_eq!(unmatched_payload["available"], false);
    assert!(
        unmatched_payload["reason"]
            .as_str()
            .ok_or("reason")?
            .contains("no LSP server matches")
    );
    Ok(())
}

#[test]
fn lsp_stdio_accepts_integer_positions_encoded_as_json_floats()
-> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let source = root.path().join("sample.toml");
    let capture = root.path().join("capture.json");
    std::fs::write(&source, "value = 1\n")?;
    let fake_lsp = env!("CARGO_BIN_EXE_codexy-fake-lsp");

    let mut client = Command::new(env!("CARGO_BIN_EXE_codexy-mcp-lsp"))
        .env("CODEXY_LSP_ALLOW_COMMAND_OVERRIDE", "1")
        .env("CODEXY_FAKE_LSP_CAPTURE", &capture)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map(|child| McpClient {
            child,
            buffer: Vec::new(),
        })?;
    let _init = client.send(&json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}))?;
    let server = json!({"id":"taplo","command":[fake_lsp]});
    let response = client.send(&json!({
        "jsonrpc":"2.0","id":2,"method":"tools/call",
        "params":{"name":"lsp_definition","arguments":{"root":root.path(),"path":"sample.toml","server":server,"line":1.0,"character":2.0,"timeoutMs":5000.0}}
    }))?;
    let payload: Value = serde_json::from_str(
        response["result"]["content"][0]["text"]
            .as_str()
            .ok_or("definition text")?,
    )?;
    assert_eq!(payload["status"], "ok");
    let capture_payload: Value = serde_json::from_str(&std::fs::read_to_string(capture)?)?;
    assert_eq!(capture_payload["position"]["line"], 1);
    assert_eq!(capture_payload["position"]["character"], 2);
    Ok(())
}
