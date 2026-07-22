use std::path::{Component, PathBuf};

pub(super) struct WorkflowScriptCommand {
    pub(super) executable: PathBuf,
    _arguments: Vec<String>,
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
        let arguments = tokens.map(str::to_owned).collect::<Vec<_>>();
        if arguments
            .iter()
            .any(|argument| !static_shell_token(argument))
        {
            return None;
        }
        Some(Self {
            executable,
            _arguments: arguments,
        })
    }
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
