pub(super) fn path_attribute(line: &str) -> Option<String> {
    let (value, suffix) = path_attribute_prefix(line)?;
    let mut comment_depth = 0;
    (is_attribute_trivia(suffix, &mut comment_depth) && comment_depth == 0).then_some(value)
}

pub(super) fn has_cfg_attr_path(line: &str) -> bool {
    let Some(attributes) = line.strip_prefix("#[cfg_attr") else {
        return false;
    };
    attributes.split(',').skip(1).any(|attribute| {
        attribute
            .trim_start()
            .strip_prefix("path")
            .is_some_and(|suffix| {
                suffix
                    .as_bytes()
                    .first()
                    .is_some_and(|byte| byte.is_ascii_whitespace() || *byte == b'=')
            })
    })
}

pub(super) fn path_attribute_prefix(line: &str) -> Option<(String, &str)> {
    let literal = line
        .strip_prefix("#[path")?
        .trim_start()
        .strip_prefix('=')?
        .trim_start();
    let (value, suffix) = string_literal(literal)?;
    let suffix = suffix.trim_start().strip_prefix(']')?;
    Some((value, suffix))
}

pub(super) fn is_attribute_trivia(line: &str, block_comment_depth: &mut usize) -> bool {
    let mut remainder = line;
    loop {
        let trimmed = remainder.trim_start();
        if *block_comment_depth == 0 {
            if trimmed.is_empty() || trimmed.starts_with("//") {
                return true;
            }
            let Some(after_comment) = trimmed.strip_prefix("/*") else {
                return false;
            };
            *block_comment_depth += 1;
            remainder = after_comment;
            continue;
        }
        let next_start = remainder.find("/*");
        let next_end = remainder.find("*/");
        match (next_start, next_end) {
            (Some(start), Some(end)) if start < end => {
                *block_comment_depth += 1;
                remainder = &remainder[start + 2..];
            }
            (_, Some(end)) => {
                *block_comment_depth -= 1;
                remainder = &remainder[end + 2..];
            }
            (Some(start), None) => {
                *block_comment_depth += 1;
                remainder = &remainder[start + 2..];
            }
            (None, None) => return true,
        }
    }
}

fn string_literal(input: &str) -> Option<(String, &str)> {
    if input.starts_with('"') {
        cooked_string(input)
    } else {
        raw_string(input)
    }
}

fn raw_string(input: &str) -> Option<(String, &str)> {
    let after_r = input.strip_prefix('r')?;
    let hashes = after_r.bytes().take_while(|byte| *byte == b'#').count();
    let opening_len = 1 + hashes + 1;
    (input.as_bytes().get(opening_len - 1) == Some(&b'"')).then_some(())?;
    let terminator = format!("\"{}", "#".repeat(hashes));
    let end = input[opening_len..].find(&terminator)? + opening_len;
    Some((
        input[opening_len..end].to_owned(),
        &input[end + terminator.len()..],
    ))
}

fn cooked_string(input: &str) -> Option<(String, &str)> {
    let mut value = String::new();
    let mut chars = input[1..].char_indices();
    while let Some((offset, character)) = chars.next() {
        match character {
            '"' => return Some((value, &input[offset + 2..])),
            '\\' => value.push(escape(&mut chars)?),
            '\r' | '\n' => return None,
            character => value.push(character),
        }
    }
    None
}

fn escape(chars: &mut std::str::CharIndices<'_>) -> Option<char> {
    match chars.next()?.1 {
        '"' => Some('"'),
        '\\' => Some('\\'),
        'n' => Some('\n'),
        'r' => Some('\r'),
        't' => Some('\t'),
        '0' => Some('\0'),
        'x' => ascii_escape(chars),
        'u' => unicode_escape(chars),
        _ => None,
    }
}

fn ascii_escape(chars: &mut std::str::CharIndices<'_>) -> Option<char> {
    let high = hex_digit(chars.next()?.1)?;
    let low = hex_digit(chars.next()?.1)?;
    let value = high * 16 + low;
    (value <= 0x7f).then_some(char::from_u32(value)?)
}

fn unicode_escape(chars: &mut std::str::CharIndices<'_>) -> Option<char> {
    (chars.next()?.1 == '{').then_some(())?;
    let mut value = 0_u32;
    let mut digits = 0;
    loop {
        match chars.next()?.1 {
            '}' if (1..=6).contains(&digits) => return char::from_u32(value),
            '_' => {}
            character if digits < 6 => {
                value = value.checked_mul(16)?.checked_add(hex_digit(character)?)?;
                digits += 1;
            }
            _ => return None,
        }
    }
}

const fn hex_digit(character: char) -> Option<u32> {
    character.to_digit(16)
}
