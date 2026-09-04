use std::io::{BufRead as _, Write as _};
use std::path::Path;
use std::process::{Command, Stdio};

use serde_json::{Value, json};

use super::TestResult;

pub(super) fn assert_inputs(
    root: &Path,
    inputs: Vec<(Value, bool)>,
    environment: &[(&str, &std::ffi::OsStr)],
) -> TestResult {
    if let Some((input, denied)) = inputs.first() {
        super::assert_input(root, input.clone(), *denied, environment)?;
    }
    let mut runner = BatchRunner::new(root)?;
    let mut result = Ok(());
    for (input, denied) in inputs {
        if let Err(error) = runner.assert_input(input, denied, environment) {
            result = Err(error);
            break;
        }
    }
    let finish = runner.finish();
    result?;
    finish
}

struct BatchRunner {
    child: std::process::Child,
    stdin: std::process::ChildStdin,
    stdout: std::io::BufReader<std::process::ChildStdout>,
}

impl BatchRunner {
    fn new(root: &Path) -> TestResult<Self> {
        let python = crate::support::executable_path("python").map_err(std::io::Error::other)?;
        let script = codexy_runtime::paths::repository_root().join(
            "packages/codexy-runtime/tests/validator_hooks/admission_runtime/batch_runner.py",
        );
        let mut command = Command::new(python);
        command
            .args(["-I", "-B"])
            .arg(script)
            .args(["--plugin-root"])
            .arg(root)
            .env_clear();
        if let Some(path) = std::env::var_os("PATH") {
            command.env("PATH", path);
        }
        command
            .env("PLUGIN_ROOT", root.as_os_str())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = command.spawn()?;
        let stdin = child.stdin.take().ok_or("batch runner stdin")?;
        let stdout = child.stdout.take().ok_or("batch runner stdout")?;
        Ok(Self {
            child,
            stdin,
            stdout: std::io::BufReader::new(stdout),
        })
    }

    fn assert_input(
        &mut self,
        input: Value,
        denied: bool,
        environment: &[(&str, &std::ffi::OsStr)],
    ) -> TestResult {
        let description = input.to_string();
        let environment = environment
            .iter()
            .map(|(key, value)| {
                Ok((
                    (*key).to_owned(),
                    value.to_str().ok_or("batch environment is not UTF-8")?.to_owned(),
                ))
            })
            .collect::<TestResult<Vec<_>>>()?
            .into_iter()
            .collect::<std::collections::BTreeMap<_, _>>();
        let mut record = serde_json::to_vec(&json!({
            "input": input,
            "environment": environment,
        }))?;
        record.push(b'\n');
        self.stdin.write_all(&record)?;
        self.stdin.flush()?;
        let mut line = String::new();
        if self.stdout.read_line(&mut line)? == 0 {
            return Err("batch runner exited before returning a result".into());
        }
        let outputs: Value = serde_json::from_str(&line)?;
        let outputs = outputs.as_array().ok_or("batch runner returned an error")?;
        assert_eq!(outputs.len(), 2, "batch runner policy count");
        let mut denials = 0;
        for output in outputs {
            let output = output.as_str().ok_or("batch runner output is not text")?;
            if output.is_empty() {
                continue;
            }
            let value: Value = serde_json::from_str(output)
                .map_err(|error| format!("invalid denial for {description}: {error}"))?;
            let decision = if input["hook_event_name"].as_str() == Some("PermissionRequest") {
                &value["hookSpecificOutput"]["decision"]["behavior"]
            } else {
                &value["hookSpecificOutput"]["permissionDecision"]
            };
            assert_eq!(decision, "deny", "{description}");
            denials += 1;
        }
        assert_eq!(denials > 0, denied, "{description}");
        Ok(())
    }

    fn finish(self) -> TestResult {
        let Self {
            mut child,
            stdin,
            stdout,
        } = self;
        drop(stdin);
        drop(stdout);
        let output = child.wait_with_output()?;
        assert!(
            output.status.success(),
            "batch runner failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        Ok(())
    }
}
