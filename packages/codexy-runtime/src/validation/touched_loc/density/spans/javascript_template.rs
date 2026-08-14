pub(super) struct Template {
    frames: Vec<Frame>,
}

enum Frame {
    Literal {
        escaped: bool,
    },
    Expression {
        depth: usize,
        quote: Option<char>,
        escaped: bool,
    },
}

impl Template {
    pub(super) fn new() -> Self {
        Self {
            frames: vec![Frame::Literal { escaped: false }],
        }
    }

    pub(super) fn strip<'a>(&mut self, mut remainder: &'a str) -> (String, Option<&'a str>) {
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
                Frame::Expression { .. } => {
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

    fn strip_literal<'a>(&mut self, line: &'a str) -> (bool, Option<&'a str>) {
        let mut escaped = matches!(self.frames.last(), Some(Frame::Literal { escaped: true }));
        for (index, character) in line.char_indices() {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '`' {
                self.frames.pop();
                return (false, Some(&line[index + 1..]));
            } else if character == '$' && line[index + 1..].starts_with('{') {
                self.frames.push(Frame::Expression {
                    depth: 1,
                    quote: None,
                    escaped: false,
                });
                return (false, Some(&line[index + 2..]));
            }
        }
        (escaped, None)
    }

    fn strip_expression<'a>(&mut self, line: &'a str) -> (String, Option<&'a str>) {
        let Some(Frame::Expression {
            depth,
            quote,
            escaped,
        }) = self.frames.last()
        else {
            unreachable!("template expression frame must be active");
        };
        let (mut depth, mut quote, mut escaped) = (*depth, *quote, *escaped);
        let mut visible = String::new();
        for (index, character) in line.char_indices() {
            if let Some(delimiter) = quote {
                if escaped {
                    escaped = false;
                } else if character == '\\' {
                    escaped = true;
                } else if character == delimiter {
                    quote = None;
                }
            } else if matches!(character, '\'' | '"') {
                quote = Some(character);
            } else if character == '`' {
                self.set_expression_state(depth, quote, escaped);
                self.frames.push(Frame::Literal { escaped: false });
                return (visible, Some(&line[index + 1..]));
            } else if character == '{' {
                depth += 1;
                visible.push(character);
            } else if character == '}' {
                depth -= 1;
                if depth == 0 {
                    self.frames.pop();
                    return (visible, Some(&line[index + 1..]));
                }
                visible.push(character);
            } else {
                visible.push(character);
            }
        }
        self.set_expression_state(depth, quote, escaped);
        (visible, None)
    }

    fn set_literal_escape(&mut self, escaped: bool) {
        if let Some(Frame::Literal { escaped: state }) = self.frames.last_mut() {
            *state = escaped;
        }
    }

    fn set_expression_state(&mut self, depth: usize, quote: Option<char>, escaped: bool) {
        if let Some(Frame::Expression {
            depth: state_depth,
            quote: state_quote,
            escaped: state_escaped,
        }) = self.frames.last_mut()
        {
            *state_depth = depth;
            *state_quote = quote;
            *state_escaped = escaped;
        }
    }
}
