use regex::Regex;

mod scope;

pub(super) fn scoped_log(
    log: &str,
    step: &serde_json::Map<String, serde_json::Value>,
    repository: &str,
    ambiguous_step_boundary: bool,
    log_group_index: usize,
) -> Result<String, String> {
    scope::scoped_log(
        log,
        step,
        repository,
        ambiguous_step_boundary,
        log_group_index,
    )
}

pub(super) struct Failure {
    pub(super) test: String,
    pub(super) path: String,
    pub(super) exception: String,
    pub(super) line: u64,
}

pub(super) fn parse_log(log: &str, repository: &str) -> Result<Failure, String> {
    let log = normalize_log(log);
    let error = Regex::new(r"(?m)^ERROR:\s*([^\s(]+)(?:\s+\(([^)]+)\))?\s*$")
        .map_err(|error| error.to_string())?;
    let errors = error.captures_iter(&log).collect::<Vec<_>>();
    if errors.len() != 1 {
        return Err("Actions log must contain exactly one supported unittest ERROR".into());
    }
    let token = &errors[0][1];
    let test = errors[0].get(2).map_or_else(
        || token.to_owned(),
        |context| {
            let context = context.as_str();
            if token.contains('.') {
                token.to_owned()
            } else if context.ends_with(&format!(".{token}")) {
                context.to_owned()
            } else {
                format!("{context}.{token}")
            }
        },
    );
    let file =
        Regex::new(r#"(?m)^\s*File "([^"]+)", line ([0-9]+)(?:, in [A-Za-z_][A-Za-z0-9_]*)?\s*$"#)
            .map_err(|error| error.to_string())?;
    let files = file.captures_iter(&log).collect::<Vec<_>>();
    let paths = files
        .iter()
        .filter(|file| is_repository_path(&file[1], repository))
        .map(|file| {
            Ok((
                repository_path(&file[1], repository)?,
                file[2]
                    .parse()
                    .map_err(|_| "Actions traceback line is invalid".to_owned())?,
            ))
        })
        .collect::<Result<Vec<_>, String>>()?;
    if paths.len() != 1 {
        return Err("Actions log must contain exactly one supported repository traceback".into());
    }
    let exception = Regex::new(r"(?m)^([A-Za-z_][A-Za-z0-9_]*(?:Error|Exception)):\s*.+$")
        .map_err(|error| error.to_string())?;
    let exceptions = exception.captures_iter(&log).collect::<Vec<_>>();
    if exceptions.len() != 1 {
        return Err("Actions log must contain exactly one supported exception".into());
    }
    let Some((path, line)) = paths.into_iter().next() else {
        return Err("Actions traceback path disappeared during projection".into());
    };
    Ok(Failure {
        test,
        path,
        exception: exceptions[0][1].to_owned(),
        line,
    })
}

fn is_repository_path(raw: &str, repository: &str) -> bool {
    let normalized = raw.replace('\\', "/");
    let name = repository.rsplit('/').next().unwrap_or_default();
    normalized.starts_with("packages/")
        || normalized.contains(&format!("/a/{name}/{name}/packages/"))
        || normalized.contains(&format!("/work/{name}/{name}/packages/"))
}

pub(super) fn normalize_log(log: &str) -> String {
    log.lines()
        .map(|line| {
            let line = strip_ansi(line);
            for marker in ["ERROR:", "Traceback (most recent call last):", "File \""] {
                if let Some(index) = line.find(marker) {
                    return line[index..].to_owned();
                }
            }
            if let Ok(exception) = Regex::new(r"[A-Za-z_][A-Za-z0-9_]*(?:Error|Exception):") {
                if let Some(found) = exception.find(&line) {
                    return line[found.start()..].to_owned();
                }
            }
            line.to_string()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn strip_ansi(line: &str) -> String {
    let mut output = String::with_capacity(line.len());
    let mut chars = line.chars();
    while let Some(character) = chars.next() {
        if character == '\x1b' && chars.next() == Some('[') {
            for control in chars.by_ref() {
                if ('@'..='~').contains(&control) {
                    break;
                }
            }
        } else {
            output.push(character);
        }
    }
    output
}

fn repository_path(raw: &str, repository: &str) -> Result<String, String> {
    let normalized = raw.replace('\\', "/");
    let name = repository.rsplit('/').next().unwrap_or_default();
    let suffix = "packages/";
    let path = if normalized.starts_with(suffix) {
        normalized
    } else if normalized.contains(&format!("/a/{name}/{name}/packages/"))
        || normalized.contains(&format!("/work/{name}/{name}/packages/"))
    {
        normalized
            .split_once(suffix)
            .map(|(_, path)| format!("packages/{path}"))
            .ok_or_else(|| "Actions traceback path is not repository-relative".to_owned())?
    } else {
        return Err("Actions traceback path is missing or absolute".into());
    };
    if path
        .split('/')
        .any(|part| part.is_empty() || matches!(part, "." | ".."))
    {
        return Err("Actions traceback path must be repository-relative".into());
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::parse_log;

    const REPOSITORY: &str = "eunsoogi/codexy";
    const PATH: &str = "packages/getcodexy/tests/test_component_capability_probe.py";

    fn log(path: &str) -> String {
        format!(
            "ERROR: packages.getcodexy.tests.test_component_capability_probe.CapabilityProcessTests.test_process_result_captures_bounded_diagnostics (packages.getcodexy.tests.test_component_capability_probe.CapabilityProcessTests)\nTraceback (most recent call last):\n  File \"{path}\", line 57, in test_process_result_captures_bounded_diagnostics\nNotImplementedError: cannot instantiate 'PosixPath' on your system\n"
        )
    }

    #[test]
    fn projects_supported_unittest_failure() {
        let finding = parse_log(&log("D:\\a\\codexy\\codexy\\packages\\getcodexy\\tests\\test_component_capability_probe.py"), REPOSITORY).unwrap();
        assert_eq!(
            finding.test,
            "packages.getcodexy.tests.test_component_capability_probe.CapabilityProcessTests.test_process_result_captures_bounded_diagnostics"
        );
        assert_eq!(finding.path, PATH);
        assert_eq!(finding.exception, "NotImplementedError");
        assert_eq!(finding.line, 57);
    }

    #[test]
    fn rejects_ambiguous_or_unsupported_output() {
        assert!(parse_log(&format!("{}\n{}", log(PATH), log(PATH)), REPOSITORY).is_err());
        assert!(
            parse_log(
                "FAIL: test_process_result_captures_bounded_diagnostics",
                REPOSITORY
            )
            .is_err()
        );
    }

    #[test]
    fn rejects_unsafe_traceback_paths() {
        assert!(
            parse_log(
                &log("D:\\a\\codexy\\codexy\\packages\\..\\secret.py"),
                REPOSITORY
            )
            .is_err()
        );
        assert!(parse_log(&log("C:\\other\\secret.py"), REPOSITORY).is_err());
    }
}
