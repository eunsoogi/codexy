use std::path::{Component, PathBuf};

pub(super) struct WorkflowScriptCommand {
    pub(super) executable: PathBuf,
    _arguments: Vec<WorkflowArgument>,
}

enum WorkflowArgument {
    Static,
    EnvironmentReference,
}

impl WorkflowScriptCommand {
    pub(super) fn parse(command: &str) -> Option<Self> {
        let mut tokens = command.split_ascii_whitespace();
        let executable = tokens.next()?;
        if !static_shell_token(executable) {
            return None;
        }
        let executable = PathBuf::from(executable);
        let components = executable.components().collect::<Vec<_>>();
        if components.len() < 2
            || components.first()?.as_os_str() != "scripts"
            || components
                .iter()
                .any(|component| !matches!(component, Component::Normal(_)))
        {
            return None;
        }
        let arguments = tokens
            .map(parse_argument)
            .collect::<Option<Vec<WorkflowArgument>>>()?;
        Some(Self {
            executable,
            _arguments: arguments,
        })
    }
}

fn parse_argument(token: &str) -> Option<WorkflowArgument> {
    if static_shell_token(token) {
        return Some(WorkflowArgument::Static);
    }
    simple_environment_reference(token).then_some(WorkflowArgument::EnvironmentReference)
}

fn simple_environment_reference(token: &str) -> bool {
    let reference = match (token.starts_with('"'), token.ends_with('"')) {
        (true, true) if token.len() > 2 => &token[1..token.len() - 1],
        (false, false) => token,
        _ => return false,
    };
    let name = if let Some(braced) = reference
        .strip_prefix("${")
        .and_then(|value| value.strip_suffix('}'))
    {
        braced
    } else if let Some(unbraced) = reference.strip_prefix('$') {
        unbraced
    } else {
        return false;
    };
    let mut bytes = name.bytes();
    bytes
        .next()
        .is_some_and(|byte| byte.is_ascii_alphabetic() || byte == b'_')
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

fn static_shell_token(token: &str) -> bool {
    !token.is_empty()
        && token.bytes().all(|byte| {
            byte.is_ascii_alphanumeric()
                || matches!(
                    byte,
                    b'-' | b'_' | b'.' | b'/' | b':' | b'=' | b',' | b'+' | b'@' | b'%'
                )
        })
}
