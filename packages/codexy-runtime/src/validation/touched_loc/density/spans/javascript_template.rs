const TEMPLATE_DELIMITER: char = '`';

pub(super) struct Template {
    frames: Vec<Frame>,
}

enum Frame {
    Literal { escaped: bool },
    Expression(Expression),
}

struct Expression {
    depth: usize,
    state: ExpressionState,
}

#[derive(Clone, Copy)]
enum ExpressionState {
    Code,
    Quote { delimiter: char, escaped: bool },
    BlockComment,
    LineComment,
    Regex { class: bool, escaped: bool },
}

impl Template {
    pub(super) fn new() -> Self {
        Self {
            frames: vec![Frame::Literal { escaped: false }],
        }
    }

    pub(super) fn strip<'a>(&mut self, mut remainder: &'a str) -> (String, Option<&'a str>) {
        self.start_line();
        let mut visible = String::new();
        loop {
            let Some(frame) = self.frames.last() else {
                return (visible, Some(remainder));
            };
            match frame {
                Frame::Literal { .. } => {
                    let (escaped, next) = self.strip_literal(remainder);
                    if let Some(tail) = next {
                        remainder = tail;
                    } else {
                        self.set_literal_escape(escaped);
                        return (visible, None);
                    }
                }
                Frame::Expression(_) => {
                    let (fragment, next) = self.strip_expression(remainder);
                    visible.push_str(&fragment);
                    if let Some(tail) = next {
                        remainder = tail;
                    } else {
                        return (visible, None);
                    }
                }
            }
        }
    }

    fn start_line(&mut self) {
        if let Some(Frame::Literal { escaped }) = self.frames.last_mut() {
            *escaped = false;
        } else if let Some(Frame::Expression(expression)) = self.frames.last_mut() {
            if matches!(expression.state, ExpressionState::LineComment) {
                expression.state = ExpressionState::Code;
            }
        }
    }

    fn strip_literal<'a>(&mut self, line: &'a str) -> (bool, Option<&'a str>) {
        let mut escaped = matches!(self.frames.last(), Some(Frame::Literal { escaped: true }));
        for (index, character) in line.char_indices() {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == TEMPLATE_DELIMITER {
                self.frames.pop();
                return (false, Some(&line[index + 1..]));
            } else if character == '$' && line[index + 1..].starts_with('{') {
                self.frames.push(Frame::Expression(Expression {
                    depth: 1,
                    state: ExpressionState::Code,
                }));
                return (false, Some(&line[index + 2..]));
            }
        }
        (escaped, None)
    }

    fn strip_expression<'a>(&mut self, line: &'a str) -> (String, Option<&'a str>) {
        let Some(Frame::Expression(expression)) = self.frames.last() else {
            unreachable!("template expression frame must be active");
        };
        let mut depth = expression.depth;
        let mut state = expression.state;
        let mut visible = String::new();
        let mut index = 0;
        while index < line.len() {
            let tail = &line[index..];
            let character = tail.chars().next().expect("index must be in bounds");
            match state {
                ExpressionState::Code if tail.starts_with("/*") => {
                    state = ExpressionState::BlockComment;
                    index += 2;
                }
                ExpressionState::Code if tail.starts_with("//") => {
                    self.set_expression_state(depth, ExpressionState::LineComment);
                    return (visible, None);
                }
                ExpressionState::Code if character == '/' && regex_context(&visible) => {
                    state = ExpressionState::Regex {
                        class: false,
                        escaped: false,
                    };
                    index += 1;
                }
                ExpressionState::Code if matches!(character, '\'' | '"') => {
                    state = ExpressionState::Quote {
                        delimiter: character,
                        escaped: false,
                    };
                    index += character.len_utf8();
                }
                ExpressionState::Code if character == TEMPLATE_DELIMITER => {
                    self.set_expression_state(depth, state);
                    self.frames.push(Frame::Literal { escaped: false });
                    return (visible, Some(&line[index + 1..]));
                }
                ExpressionState::Code if character == '{' => {
                    depth += 1;
                    visible.push(character);
                    index += 1;
                }
                ExpressionState::Code if character == '}' => {
                    depth -= 1;
                    if depth == 0 {
                        self.frames.pop();
                        return (visible, Some(&line[index + 1..]));
                    }
                    visible.push(character);
                    index += 1;
                }
                ExpressionState::Code => {
                    visible.push(character);
                    index += character.len_utf8();
                }
                ExpressionState::Quote { delimiter, escaped } => {
                    state = if escaped {
                        ExpressionState::Quote {
                            delimiter,
                            escaped: false,
                        }
                    } else if character == '\\' {
                        ExpressionState::Quote {
                            delimiter,
                            escaped: true,
                        }
                    } else if character == delimiter {
                        ExpressionState::Code
                    } else {
                        state
                    };
                    index += character.len_utf8();
                }
                ExpressionState::BlockComment if tail.starts_with("*/") => {
                    state = ExpressionState::Code;
                    index += 2;
                }
                ExpressionState::BlockComment => index += character.len_utf8(),
                ExpressionState::LineComment => {
                    self.set_expression_state(depth, state);
                    return (visible, None);
                }
                ExpressionState::Regex { class, escaped } => {
                    state = if escaped {
                        ExpressionState::Regex {
                            class,
                            escaped: false,
                        }
                    } else if character == '\\' {
                        ExpressionState::Regex {
                            class,
                            escaped: true,
                        }
                    } else if character == '[' {
                        ExpressionState::Regex {
                            class: true,
                            escaped: false,
                        }
                    } else if character == ']' {
                        ExpressionState::Regex {
                            class: false,
                            escaped: false,
                        }
                    } else if character == '/' && !class {
                        ExpressionState::Code
                    } else {
                        state
                    };
                    index += character.len_utf8();
                }
            }
        }
        self.set_expression_state(depth, state);
        (visible, None)
    }

    fn set_literal_escape(&mut self, escaped: bool) {
        if let Some(Frame::Literal { escaped: state }) = self.frames.last_mut() {
            *state = escaped;
        }
    }

    fn set_expression_state(&mut self, depth: usize, state: ExpressionState) {
        if let Some(Frame::Expression(expression)) = self.frames.last_mut() {
            expression.depth = depth;
            expression.state = state;
        }
    }
}

fn regex_context(prefix: &str) -> bool {
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
