use std::path::{Component, PathBuf};

pub(super) struct WorkflowScriptCommand {
    pub(super) executable: PathBuf,
    _arguments: Vec<WorkflowArgument>,
}

enum WorkflowArgument {
    Static,
    EnvironmentReference,
    GitHubExpression,
}

enum WorkflowWordSegment {
    Unquoted(String),
    SingleQuoted(String),
    DoubleQuoted(String),
}

struct WorkflowWord {
    segments: Vec<WorkflowWordSegment>,
}

impl WorkflowScriptCommand {
    pub(super) fn parse(command: &str) -> Option<Self> {
        let mut words = lex_command(command)?.into_iter();
        let executable = parse_executable(words.next()?)?;
        let components = executable.components().collect::<Vec<_>>();
        if components.len() < 2
            || components.first()?.as_os_str() != "scripts"
            || components
                .iter()
                .any(|component| !matches!(component, Component::Normal(_)))
        {
            return None;
        }
        let arguments = words
            .map(parse_argument)
            .collect::<Option<Vec<WorkflowArgument>>>()?;
        Some(Self {
            executable,
            _arguments: arguments,
        })
    }
}

fn parse_executable(word: WorkflowWord) -> Option<PathBuf> {
    let [WorkflowWordSegment::Unquoted(executable)] = word.segments.as_slice() else {
        return None;
    };
    static_shell_token(executable).then(|| PathBuf::from(executable))
}

fn parse_argument(word: WorkflowWord) -> Option<WorkflowArgument> {
    match word.segments.as_slice() {
        [WorkflowWordSegment::Unquoted(value)] if static_shell_token(value) => {
            Some(WorkflowArgument::Static)
        }
        [WorkflowWordSegment::Unquoted(value) | WorkflowWordSegment::DoubleQuoted(value)]
            if simple_environment_reference(value) =>
        {
            Some(WorkflowArgument::EnvironmentReference)
        }
        [WorkflowWordSegment::SingleQuoted(value) | WorkflowWordSegment::DoubleQuoted(value)]
            if static_quoted_text(value) =>
        {
            Some(WorkflowArgument::Static)
        }
        [WorkflowWordSegment::DoubleQuoted(value)] if quoted_github_expression(value) => {
            Some(WorkflowArgument::GitHubExpression)
        }
        _ => None,
    }
}

fn simple_environment_reference(value: &str) -> bool {
    let name = if let Some(braced) = value
        .strip_prefix("${")
        .and_then(|value| value.strip_suffix('}'))
    {
        braced
    } else if let Some(unbraced) = value.strip_prefix('$') {
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

fn quoted_github_expression(value: &str) -> bool {
    let mut remaining = value;
    let mut found = false;
    while let Some(start) = remaining.find("${{") {
        if !static_quoted_fragment(&remaining[..start]) {
            return false;
        }
        let expression_and_suffix = &remaining[start + 3..];
        let Some(end) = expression_and_suffix.find("}}") else {
            return false;
        };
        let expression = &expression_and_suffix[..end];
        if expression.contains("${{") || !github_expression_name(expression) {
            return false;
        }
        found = true;
        remaining = &expression_and_suffix[end + 2..];
    }
    found && !remaining.contains("}}") && static_quoted_fragment(remaining)
}

fn github_expression_name(expression: &str) -> bool {
    let expression = expression.trim();
    !expression.is_empty() && expression.split('.').all(shell_identifier)
}

fn shell_identifier(value: &str) -> bool {
    let mut bytes = value.bytes();
    bytes
        .next()
        .is_some_and(|byte| byte.is_ascii_alphabetic() || byte == b'_')
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

fn static_quoted_text(value: &str) -> bool {
    !value.is_empty() && static_quoted_fragment(value)
}

fn static_quoted_fragment(value: &str) -> bool {
    value
        .bytes()
        .all(|byte| static_shell_byte(byte) || matches!(byte, b' ' | b'\t'))
}

fn lex_command(command: &str) -> Option<Vec<WorkflowWord>> {
    if command.is_empty()
        || command
            .bytes()
            .any(|byte| matches!(byte, b'\n' | b'\r' | b'\\'))
    {
        return None;
    }
    let mut words = Vec::new();
    let mut segments = Vec::new();
    let mut unquoted = String::new();
    let mut quoted: Option<(char, String)> = None;
    for character in command.chars() {
        if let Some((delimiter, value)) = quoted.as_mut() {
            if character == *delimiter {
                let delimiter = *delimiter;
                let (_, value) = quoted.take()?;
                segments.push(if delimiter == '\'' {
                    WorkflowWordSegment::SingleQuoted(value)
                } else {
                    WorkflowWordSegment::DoubleQuoted(value)
                });
            } else {
                value.push(character);
            }
            continue;
        }
        match character {
            '\'' | '"' => {
                push_unquoted_segment(&mut segments, &mut unquoted);
                quoted = Some((character, String::new()));
            }
            character if character.is_ascii_whitespace() => {
                push_unquoted_segment(&mut segments, &mut unquoted);
                push_word(&mut words, &mut segments);
            }
            _ => unquoted.push(character),
        }
    }
    if quoted.is_some() {
        return None;
    }
    push_unquoted_segment(&mut segments, &mut unquoted);
    push_word(&mut words, &mut segments);
    (!words.is_empty()).then_some(words)
}

fn push_unquoted_segment(segments: &mut Vec<WorkflowWordSegment>, value: &mut String) {
    if !value.is_empty() {
        segments.push(WorkflowWordSegment::Unquoted(std::mem::take(value)));
    }
}

fn push_word(words: &mut Vec<WorkflowWord>, segments: &mut Vec<WorkflowWordSegment>) {
    if !segments.is_empty() {
        words.push(WorkflowWord {
            segments: std::mem::take(segments),
        });
    }
}

fn static_shell_token(token: &str) -> bool {
    !token.is_empty() && token.bytes().all(static_shell_byte)
}

fn static_shell_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric()
        || matches!(
            byte,
            b'-' | b'_' | b'.' | b'/' | b':' | b'=' | b',' | b'+' | b'@' | b'%'
        )
}
