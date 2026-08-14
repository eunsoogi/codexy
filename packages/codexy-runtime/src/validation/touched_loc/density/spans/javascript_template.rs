pub(super) fn strip_template<'a>(
    mut remainder: &'a str,
    depth: &mut usize,
    quote: &mut Option<char>,
    escaped: &mut bool,
) -> (String, Option<&'a str>) {
    let mut visible = String::new();
    loop {
        if *depth == 0 {
            let Some((index, boundary)) = template_boundary(remainder, escaped) else {
                return (visible, None);
            };
            match boundary {
                TemplateBoundary::End => return (visible, Some(&remainder[index + 1..])),
                TemplateBoundary::Interpolation => {
                    remainder = &remainder[index + 2..];
                    *depth = 1;
                }
            }
        } else {
            let (expression, tail) = template_expression(remainder, depth, quote, escaped);
            visible.push_str(&expression);
            let Some(tail) = tail else {
                return (visible, None);
            };
            remainder = tail;
        }
    }
}

enum TemplateBoundary {
    End,
    Interpolation,
}

fn template_boundary(line: &str, escaped: &mut bool) -> Option<(usize, TemplateBoundary)> {
    for (index, character) in line.char_indices() {
        if *escaped {
            *escaped = false;
        } else if character == '\\' {
            *escaped = true;
        } else if character == '`' {
            return Some((index, TemplateBoundary::End));
        } else if line[index..].starts_with("${") {
            return Some((index, TemplateBoundary::Interpolation));
        }
    }
    None
}

fn template_expression<'a>(
    line: &'a str,
    depth: &mut usize,
    quote: &mut Option<char>,
    escaped: &mut bool,
) -> (String, Option<&'a str>) {
    let mut visible = String::new();
    for (index, character) in line.char_indices() {
        if let Some(delimiter) = quote {
            if *escaped {
                *escaped = false;
            } else if character == '\\' {
                *escaped = true;
            } else if character == *delimiter {
                *quote = None;
            }
        } else if matches!(character, '\'' | '"' | '`') {
            *quote = Some(character);
        } else if character == '{' {
            *depth += 1;
            visible.push(character);
        } else if character == '}' {
            *depth -= 1;
            if *depth == 0 {
                return (visible, Some(&line[index + 1..]));
            }
            visible.push(character);
        } else {
            visible.push(character);
        }
    }
    (visible, None)
}
