#[derive(Clone, Copy)]
pub(super) enum ExpressionState {
    Code,
    Quote { delimiter: char, escaped: bool },
    BlockComment,
    LineComment,
    Regex { class: bool, escaped: bool },
}

impl ExpressionState {
    pub(super) fn start_line(self) -> Self {
        match self {
            Self::LineComment => Self::Code,
            Self::Quote { delimiter, .. } => Self::Quote {
                delimiter,
                escaped: false,
            },
            Self::Regex { class, .. } => Self::Regex {
                class,
                escaped: false,
            },
            state => state,
        }
    }
}

pub(super) fn regex_context(prefix: &str) -> bool {
    let trimmed = prefix.trim_end();
    matches!(
        trimmed.split_whitespace().next_back(),
        Some("return" | "throw" | "case" | "yield")
    ) || trimmed.ends_with("=>")
        || trimmed.chars().next_back().is_none_or(|character| {
            matches!(
                character,
                '=' | '(' | '[' | '{' | ',' | ':' | ';' | '!' | '&' | '|' | '?'
            )
        })
}
