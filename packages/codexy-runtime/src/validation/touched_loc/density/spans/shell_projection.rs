pub(super) struct Projection {
    frames: Vec<Frame>,
}

enum Frame {
    Quote(Quote),
    Substitution { depth: usize },
}

#[derive(Clone, Copy)]
struct Quote {
    delimiter: char,
    escaped: bool,
}

impl Projection {
    pub(super) fn new() -> Self {
        Self { frames: Vec::new() }
    }

    pub(super) fn project(&mut self, line: &str) -> String {
        self.start_line();
        let mut visible = String::with_capacity(line.len());
        let mut index = 0;
        while index < line.len() {
            let tail = &line[index..];
            let character = tail.chars().next().expect("index must be in bounds");
            match self.frames.last_mut() {
                Some(Frame::Quote(quote)) => {
                    if quote.escaped {
                        quote.escaped = false;
                        mask(&mut visible, character);
                        index += character.len_utf8();
                    } else if quote.delimiter == '"' && character == '\\' {
                        quote.escaped = true;
                        mask(&mut visible, character);
                        index += 1;
                    } else if quote.delimiter == '"'
                        && tail.starts_with("$(")
                        && !tail.starts_with("$((")
                    {
                        mask_pair(&mut visible);
                        self.frames.push(Frame::Substitution { depth: 1 });
                        index += 2;
                    } else if character == quote.delimiter {
                        self.frames.pop();
                        mask(&mut visible, character);
                        index += character.len_utf8();
                    } else {
                        mask(&mut visible, character);
                        index += character.len_utf8();
                    }
                }
                Some(Frame::Substitution { depth }) => {
                    if matches!(character, '\'' | '"') {
                        self.frames.push(Frame::Quote(Quote {
                            delimiter: character,
                            escaped: false,
                        }));
                        mask(&mut visible, character);
                        index += character.len_utf8();
                    } else if tail.starts_with("$(") && !tail.starts_with("$((") {
                        mask_pair(&mut visible);
                        self.frames.push(Frame::Substitution { depth: 1 });
                        index += 2;
                    } else if character == '(' {
                        *depth += 1;
                        visible.push(character);
                        index += 1;
                    } else if character == ')' {
                        *depth -= 1;
                        mask(&mut visible, character);
                        index += 1;
                        if *depth == 0 {
                            self.frames.pop();
                        }
                    } else {
                        visible.push(character);
                        index += character.len_utf8();
                    }
                }
                None if matches!(character, '\'' | '"') => {
                    self.frames.push(Frame::Quote(Quote {
                        delimiter: character,
                        escaped: false,
                    }));
                    mask(&mut visible, character);
                    index += character.len_utf8();
                }
                None if tail.starts_with("$(") && !tail.starts_with("$((") => {
                    self.frames.push(Frame::Substitution { depth: 1 });
                    mask_pair(&mut visible);
                    index += 2;
                }
                None => {
                    visible.push(character);
                    index += character.len_utf8();
                }
            }
        }
        visible
    }

    fn start_line(&mut self) {
        for frame in &mut self.frames {
            if let Frame::Quote(quote) = frame {
                quote.escaped = false;
            }
        }
    }
}

pub(super) fn continues(line: &str) -> bool {
    line.as_bytes()
        .iter()
        .rev()
        .take_while(|byte| **byte == b'\\')
        .count()
        % 2
        == 1
}

fn mask(visible: &mut String, character: char) {
    visible.push_str(&" ".repeat(character.len_utf8()));
}

fn mask_pair(visible: &mut String) {
    visible.push_str("  ");
}
