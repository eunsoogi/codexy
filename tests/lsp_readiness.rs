use std::io::{BufRead as _, BufReader, Write as _};
use std::process::{Child, Command, Stdio};

use serde_json::{Value, json};

struct McpClient {
    child: Child,
    _path_dir: tempfile::TempDir,
}

impl McpClient {
    fn spawn() -> Result<Self, Box<dyn std::error::Error>> {
        let path_dir = tempfile::tempdir()?;
        let child = Command::new(env!("CARGO_BIN_EXE_codexy-mcp-lsp"))
            .env("PATH", path_dir.path())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        Ok(Self {
            child,
            _path_dir: path_dir,
        })
    }

    fn send(&mut self, payload: &Value) -> Result<Value, Box<dyn std::error::Error>> {
        let body = serde_json::to_vec(payload)?;
        let stdin = self.child.stdin.as_mut().ok_or("missing child stdin")?;
        stdin.write_all(&body)?;
        stdin.write_all(b"\n")?;
        stdin.flush()?;
        let stdout = self.child.stdout.as_mut().ok_or("missing child stdout")?;
        let mut line = String::new();
        BufReader::new(stdout).read_line(&mut line)?;
        Ok(serde_json::from_str(&line)?)
    }
}

#[test]
fn lsp_status_classifies_missing_rust_analyzer_as_readiness_defect()
-> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    std::fs::write(root.path().join("sample.rs"), "fn main() {}\n")?;

    let mut client = McpClient::spawn()?;
    let init = client.send(&json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}))?;
    assert_eq!(init["result"]["serverInfo"]["name"], "codexy-lsp");

    let status = client.send(&json!({
        "jsonrpc":"2.0","id":2,"method":"tools/call",
        "params":{"name":"lsp_status","arguments":{"root":root.path(),"path":"sample.rs"}}
    }))?;
    let status_payload: Value = serde_json::from_str(
        status["result"]["content"][0]["text"]
            .as_str()
            .ok_or("text")?,
    )?;

    assert_eq!(status_payload["server"]["id"], "rust-analyzer");
    assert_eq!(status_payload["available"], false);
    assert_eq!(status_payload["readiness"]["defect"], "missing-executable");
    assert_eq!(
        status_payload["readiness"]["action"],
        "install rust-analyzer or put it on PATH before relying on Rust LSP diagnostics"
    );
    assert!(
        status_payload["reason"]
            .as_str()
            .ok_or("reason")?
            .contains("executable not found on PATH: rust-analyzer")
    );
    Ok(())
}

#[test]
fn lsp_status_matches_html_to_web_language_server() -> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    std::fs::write(root.path().join("index.html"), "<main>Hello</main>\n")?;

    let mut client = McpClient::spawn()?;
    let init = client.send(&json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}))?;
    assert_eq!(init["result"]["serverInfo"]["name"], "codexy-lsp");

    let status = client.send(&json!({
        "jsonrpc":"2.0","id":2,"method":"tools/call",
        "params":{"name":"lsp_status","arguments":{"root":root.path(),"path":"index.html"}}
    }))?;
    let status_payload: Value = serde_json::from_str(
        status["result"]["content"][0]["text"]
            .as_str()
            .ok_or("text")?,
    )?;

    assert_ne!(status_payload["server"]["id"], "unmatched");
    assert_eq!(status_payload["server"]["id"], "html-language-server");
    assert_eq!(status_payload["server"]["language"], "HTML");
    assert_eq!(status_payload["extension"], ".html");
    Ok(())
}

#[test]
fn lsp_status_preserves_scss_and_less_language_ids() -> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    std::fs::write(root.path().join("styles.scss"), "$color: #111;\n")?;
    std::fs::write(root.path().join("styles.less"), "@color: #111;\n")?;

    let mut client = McpClient::spawn()?;
    let init = client.send(&json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}))?;
    assert_eq!(init["result"]["serverInfo"]["name"], "codexy-lsp");

    for (id, path, extension, language) in [
        ("scss", "styles.scss", ".scss", "scss"),
        ("less", "styles.less", ".less", "less"),
    ] {
        let status = client.send(&json!({
            "jsonrpc":"2.0","id":id,"method":"tools/call",
            "params":{"name":"lsp_status","arguments":{"root":root.path(),"path":path}}
        }))?;
        let status_payload: Value = serde_json::from_str(
            status["result"]["content"][0]["text"]
                .as_str()
                .ok_or("text")?,
        )?;

        assert_eq!(status_payload["server"]["id"], "css-language-server");
        assert_eq!(status_payload["extension"], extension);
        assert_eq!(status_payload["language"], language);
    }
    Ok(())
}

#[test]
fn lsp_config_covers_core_web_and_content_extensions() -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .arg("--print-covered-extensions")
        .output()?;
    assert!(
        output.status.success(),
        "covered extension listing failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout)?;
    for extension in [
        ".css", ".graphql", ".gql", ".html", ".less", ".scss", ".vue", ".yaml", ".yml",
    ] {
        assert!(
            stdout.lines().any(|line| line == extension),
            "LSP coverage must include {extension}, got:\n{stdout}"
        );
    }
    Ok(())
}

#[test]
fn lsp_validator_rejects_missing_required_web_extension() -> Result<(), Box<dyn std::error::Error>>
{
    let root = tempfile::tempdir()?;
    std::fs::create_dir_all(root.path().join(".codex"))?;
    std::fs::create_dir_all(root.path().join("lsp"))?;
    std::fs::copy(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("plugins/codexy/lsp/server-catalog.toml"),
        root.path().join("lsp/server-catalog.toml"),
    )?;
    let config_path = root.path().join(".codex/lsp-client.json");
    let mut config: Value = serde_json::from_str(&std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("plugins/codexy/.codex/lsp-client.json"),
    )?)?;
    let graphql_extensions = config["lsp"]["graphql-language-service"]["extensions"]
        .as_array_mut()
        .ok_or("graphql extensions must be array")?;
    graphql_extensions.retain(|extension| extension != ".graphql");
    std::fs::write(&config_path, serde_json::to_vec_pretty(&config)?)?;

    let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .arg("--plugin-root")
        .arg(root.path())
        .arg("--check-lsp")
        .output()?;
    assert!(
        !output.status.success(),
        "validator should reject missing required .graphql coverage"
    );
    let stderr = String::from_utf8(output.stderr)?;
    assert!(
        stderr.contains("LSP coverage missing required extensions: .graphql"),
        "validator failure should name missing .graphql coverage, got {stderr}"
    );
    Ok(())
}
