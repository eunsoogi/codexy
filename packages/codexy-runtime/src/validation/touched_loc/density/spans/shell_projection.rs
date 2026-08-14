#[derive(Clone)]
pub(super) struct Projection {
    frames: Vec<Frame>,
}

#[derive(Clone)]
enum Frame {
    Quote(Quote),
    Substitution { depth: usize },
    Arithmetic { depth: usize },
    Escape,
}

#[derive(Clone, Copy)]
struct Quote {
    delimiter: char,
    escaped: bool,
}

pub(super) struct Projected {
    pub(super) visible: String,
    pub(super) continues: bool,
}

impl Projection {
    pub(super) fn new() -> Self {
        Self { frames: Vec::new() }
    }

    pub(super) fn continues(&self, line: &str) -> bool {
        let mut probe = self.clone();
        probe.project(line).continues
    }

    pub(super) fn project(&mut self, line: &str) -> Projected {
        self.start_line();
        let mut visible = String::with_capacity(line.len());
        let mut comment_boundary = true;
        let mut continues = false;
        let mut index = 0;
        while index < line.len() {
            let tail = &line[index..];
            let character = tail.chars().next().expect("index must be in bounds");
            match self.frames.last_mut() {
                Some(Frame::Escape) => {
                    self.frames.pop();
                    comment_boundary = false;
                    mask(&mut visible, character);
                    index += character.len_utf8();
                }
                Some(Frame::Quote(quote)) => {
                    comment_boundary = false;
                    if quote.escaped {
                        quote.escaped = false;
                        mask(&mut visible, character);
                        index += character.len_utf8();
                    } else if quote.delimiter == '"' && character == '\\' {
                        quote.escaped = true;
                        continues = tail == "\\";
                        mask(&mut visible, character);
                        index += 1;
                    } else if quote.delimiter == '"' && arithmetic_start(tail).is_some() {
                        self.start_arithmetic(tail, &mut visible, &mut index);
                    } else if quote.delimiter == '"' && tail.starts_with("$(") {
                        self.start_substitution(&mut visible, &mut index);
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
                    if character == '\\' {
                        comment_boundary = false;
                        continues = tail == "\\";
                        self.start_escape(&mut visible, &mut index);
                    } else if matches!(character, '\'' | '"') {
                        comment_boundary = false;
                        self.start_quote(character, &mut visible, &mut index);
                    } else if arithmetic_start(tail).is_some() {
                        comment_boundary = false;
                        self.start_arithmetic(tail, &mut visible, &mut index);
                    } else if tail.starts_with("$(") {
                        comment_boundary = false;
                        self.start_substitution(&mut visible, &mut index);
                    } else if character == '(' {
                        comment_boundary = true;
                        *depth += 1;
                        visible.push(character);
                        index += 1;
                    } else if character == ')' {
                        comment_boundary = false;
                        *depth -= 1;
                        mask(&mut visible, character);
                        index += 1;
                        if *depth == 0 {
                            self.frames.pop();
                        }
                    } else if character == '#' && comment_boundary {
                        mask_rest(&mut visible, tail);
                        break;
                    } else {
                        visible.push(character);
                        update_comment_boundary(&mut comment_boundary, character);
                        index += character.len_utf8();
                    }
                }
                Some(Frame::Arithmetic { depth }) => {
                    if character == '\\' {
                        comment_boundary = false;
                        continues = tail == "\\";
                        self.start_escape(&mut visible, &mut index);
                    } else if matches!(character, '\'' | '"') {
                        comment_boundary = false;
                        self.start_quote(character, &mut visible, &mut index);
                    } else if arithmetic_start(tail).is_some() {
                        comment_boundary = false;
                        self.start_arithmetic(tail, &mut visible, &mut index);
                    } else if tail.starts_with("$(") {
                        comment_boundary = false;
                        self.start_substitution(&mut visible, &mut index);
                    } else if tail.starts_with("))") {
                        comment_boundary = false;
                        *depth -= 1;
                        mask_count(&mut visible, 2);
                        index += 2;
                        if *depth == 0 {
                            self.frames.pop();
                        }
                    } else if tail.starts_with("<<<") {
                        comment_boundary = false;
                        mask_count(&mut visible, 3);
                        index += 3;
                    } else if tail.starts_with("<<") {
                        comment_boundary = false;
                        mask_count(&mut visible, 2);
                        index += 2;
                    } else {
                        visible.push(character);
                        comment_boundary = false;
                        index += character.len_utf8();
                    }
                }
                None if character == '\\' => {
                    comment_boundary = false;
                    continues = tail == "\\";
                    self.start_escape(&mut visible, &mut index);
                }
                None if matches!(character, '\'' | '"') => {
                    comment_boundary = false;
                    self.start_quote(character, &mut visible, &mut index);
                }
                None if arithmetic_start(tail).is_some() => {
                    comment_boundary = false;
                    self.start_arithmetic(tail, &mut visible, &mut index);
                }
                None if tail.starts_with("$(") => {
                    comment_boundary = false;
                    self.start_substitution(&mut visible, &mut index);
                }
                None if character == '#' && comment_boundary => {
                    mask_rest(&mut visible, tail);
                    break;
                }
                None => {
                    visible.push(character);
                    update_comment_boundary(&mut comment_boundary, character);
                    index += character.len_utf8();
                }
            }
        }
        Projected { visible, continues }
    }

    fn start_line(&mut self) {
        for frame in &mut self.frames {
            if let Frame::Quote(quote) = frame {
                quote.escaped = false;
            }
        }
    }

    fn start_escape(&mut self, visible: &mut String, index: &mut usize) {
        self.frames.push(Frame::Escape);
        mask_count(visible, 1);
        *index += 1;
    }

    fn start_quote(&mut self, delimiter: char, visible: &mut String, index: &mut usize) {
        self.frames.push(Frame::Quote(Quote {
            delimiter,
            escaped: false,
        }));
        mask(visible, delimiter);
        *index += delimiter.len_utf8();
    }

    fn start_substitution(&mut self, visible: &mut String, index: &mut usize) {
        self.frames.push(Frame::Substitution { depth: 1 });
        mask_count(visible, 2);
        *index += 2;
    }

    fn start_arithmetic(&mut self, tail: &str, visible: &mut String, index: &mut usize) {
        let length = arithmetic_start(tail).expect("arithmetic opener must be present");
        self.frames.push(Frame::Arithmetic { depth: 1 });
        mask_count(visible, length);
        *index += length;
    }
}

fn arithmetic_start(tail: &str) -> Option<usize> {
    if tail.starts_with("$((") {
        Some(3)
    } else if tail.starts_with("((") {
        Some(2)
    } else {
        None
    }
}

fn update_comment_boundary(boundary: &mut bool, character: char) {
    *boundary = character.is_whitespace() || matches!(character, ';' | '&' | '|');
}

fn mask(visible: &mut String, character: char) {
    mask_count(visible, character.len_utf8());
}

fn mask_count(visible: &mut String, count: usize) {
    visible.push_str(&" ".repeat(count));
}

fn mask_rest(visible: &mut String, text: &str) {
    mask_count(visible, text.len());
}
