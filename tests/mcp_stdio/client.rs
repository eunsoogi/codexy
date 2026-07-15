use super::*;

pub(super) struct InstalledPlugin {
    pub(super) _temp: tempfile::TempDir,
    pub(super) path: PathBuf,
}

pub(super) struct TempRuntimeDir {
    pub(super) _temp: tempfile::TempDir,
    pub(super) path: PathBuf,
}

pub(super) struct McpClient {
    pub(super) child: Child,
    pub(super) buffer: Vec<u8>,
}

impl McpClient {
    pub(super) fn spawn(binary: &str) -> Result<Self, Box<dyn std::error::Error>> {
        Self::spawn_with(binary, None)
    }

    pub(super) fn spawn_in(binary: &str, cwd: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        Self::spawn_with(binary, Some(cwd))
    }

    pub(super) fn spawn_with(
        binary: &str,
        cwd: Option<&Path>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let mut command = Command::new(binary);
        if let Some(cwd) = cwd {
            command.current_dir(cwd);
        }
        Self::spawn_command(command)
    }

    pub(super) fn spawn_command(mut command: Command) -> Result<Self, Box<dyn std::error::Error>> {
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

    pub(super) fn send(&mut self, payload: &Value) -> Result<Value, Box<dyn std::error::Error>> {
        let body = serde_json::to_vec(&payload)?;
        let stdin = self.child.stdin.as_mut().ok_or("missing child stdin")?;
        write!(stdin, "Content-Length: {}\r\n\r\n", body.len())?;
        stdin.write_all(&body)?;
        stdin.flush()?;
        self.read_frame()
    }

    pub(super) fn send_with_leading_content_type(
        &mut self,
        payload: &Value,
    ) -> Result<Value, Box<dyn std::error::Error>> {
        let body = serde_json::to_vec(&payload)?;
        let stdin = self.child.stdin.as_mut().ok_or("missing child stdin")?;
        write!(
            stdin,
            "Content-Type: application/json\r\nContent-Length: {}\r\n\r\n",
            body.len()
        )?;
        stdin.write_all(&body)?;
        stdin.flush()?;
        self.read_frame()
    }

    pub(super) fn send_line(
        &mut self,
        payload: &Value,
    ) -> Result<Value, Box<dyn std::error::Error>> {
        let body = serde_json::to_vec(&payload)?;
        let stdin = self.child.stdin.as_mut().ok_or("missing child stdin")?;
        stdin.write_all(&body)?;
        stdin.write_all(b"\n")?;
        stdin.flush()?;
        self.read_frame()
    }

    pub(super) fn read_frame(&mut self) -> Result<Value, Box<dyn std::error::Error>> {
        loop {
            if self
                .buffer
                .get(..15)
                .is_some_and(|prefix| prefix.eq_ignore_ascii_case(b"content-length:"))
            {
                let Some(header_end) = self
                    .buffer
                    .windows(4)
                    .position(|window| window == b"\r\n\r\n")
                else {
                    self.read_stdout_chunk()?;
                    continue;
                };
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
            } else if let Some(line_end) = self.buffer.iter().position(|byte| *byte == b'\n') {
                let mut line = self.buffer.drain(..=line_end).collect::<Vec<_>>();
                while matches!(line.last(), Some(b'\n' | b'\r')) {
                    line.pop();
                }
                if line.is_empty() {
                    continue;
                }
                return Ok(serde_json::from_slice(&line)?);
            }
            self.read_stdout_chunk()?;
        }
    }

    pub(super) fn read_stdout_chunk(&mut self) -> Result<(), Box<dyn std::error::Error>> {
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
        Ok(())
    }
}
impl Drop for McpClient {
    fn drop(&mut self) {
        drop(self.child.stdin.take());
        let _ = self.child.wait();
    }
}
