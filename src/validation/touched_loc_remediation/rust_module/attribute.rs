pub(super) mod trivia;

use trivia::{attribute_name_suffix, is_attribute_trivia};

pub(super) fn path_attribute(line: &str) -> Option<String> {
    let (value, suffix) = path_attribute_prefix(line)?;
    let mut comment_depth = 0;
    (is_attribute_trivia(suffix, &mut comment_depth) && comment_depth == 0).then_some(value)
}

pub(super) fn has_cfg_attr_path(line: &str) -> bool {
    let Some(arguments) = named_attribute_content(line, "cfg_attr") else {
        return false;
    };
    let Some(arguments) = arguments.trim_start().strip_prefix('(') else {
        return true;
    };
    let mut index = 0;
    let mut argument_start = 0;
    let mut argument_index = 0;
    let mut delimiters = Vec::new();
    while index < arguments.len() {
        if let Some(next) = comment_end(arguments.as_bytes(), index) {
            let Some(next) = next else {
                return true;
            };
            index = next;
            continue;
        }
        if matches!(arguments.as_bytes()[index], b'"' | b'r') {
            if let Some((_, suffix)) = string_literal(&arguments[index..]) {
                index = arguments.len() - suffix.len();
                continue;
            }
            if arguments.as_bytes()[index] == b'"' {
                return true;
            }
        }
        match arguments.as_bytes()[index] {
            b'(' => delimiters.push(b')'),
            b'[' => delimiters.push(b']'),
            b'{' => delimiters.push(b'}'),
            b')' if delimiters.is_empty() => {
                return argument_index > 0
                    && cfg_attr_path_argument(&arguments[argument_start..index]);
            }
            b')' | b']' | b'}' => {
                if delimiters.pop() != Some(arguments.as_bytes()[index]) {
                    return true;
                }
            }
            b',' if delimiters.is_empty() => {
                if argument_index > 0 && cfg_attr_path_argument(&arguments[argument_start..index]) {
                    return true;
                }
                argument_index += 1;
                argument_start = index + 1;
            }
            _ => {}
        }
        index += 1;
    }
    true
}

fn cfg_attr_path_argument(argument: &str) -> bool {
    let Some(suffix) = argument.trim_start().strip_prefix("path") else {
        return false;
    };
    match suffix.as_bytes().first() {
        None | Some(b'=') => true,
        Some(byte) if byte.is_ascii_whitespace() => true,
        Some(b'/') => comment_end(suffix.as_bytes(), 0).is_some(),
        _ => false,
    }
}

pub(super) fn path_attribute_prefix(line: &str) -> Option<(String, &str)> {
    let literal = named_attribute_content(line, "path")?
        .trim_start()
        .strip_prefix('=')?
        .trim_start();
    let (value, suffix) = string_literal(literal)?;
    let suffix = suffix.trim_start().strip_prefix(']')?;
    Some((value, suffix))
}

pub(super) fn is_path_attribute_start(source: &str) -> bool {
    outer_attribute_content(source)
        .and_then(|content| content.strip_prefix("path"))
        .is_some_and(|suffix| {
            suffix.as_bytes().first().is_none_or(|byte| {
                byte.is_ascii_whitespace() || matches!(byte, b'=' | b']' | b'(' | b'/')
            })
        })
}

pub(super) fn is_outer_attribute(source: &str) -> bool {
    outer_attribute_content(source).is_some()
}

fn named_attribute_content<'a>(source: &'a str, name: &str) -> Option<&'a str> {
    let remainder = outer_attribute_content(source)?.strip_prefix(name)?;
    let suffix = attribute_name_suffix(remainder)?;
    (suffix.len() != remainder.len()
        || suffix
            .as_bytes()
            .first()
            .is_none_or(|byte| byte.is_ascii_whitespace() || matches!(byte, b'=' | b']' | b'(')))
    .then_some(suffix)
}

fn outer_attribute_content(source: &str) -> Option<&str> {
    source
        .strip_prefix('#')?
        .trim_start()
        .strip_prefix('[')
        .map(str::trim_start)
}

fn string_literal(input: &str) -> Option<(String, &str)> {
    if input.starts_with('"') {
        cooked_string(input)
    } else {
        raw_string(input)
    }
}

fn comment_end(bytes: &[u8], index: usize) -> Option<Option<usize>> {
    if bytes.get(index..index + 2) == Some(b"//") {
        return Some(Some(
            bytes[index + 2..]
                .iter()
                .position(|byte| *byte == b'\n')
                .map_or(bytes.len(), |offset| index + offset + 3),
        ));
    }
    if bytes.get(index..index + 2) != Some(b"/*") {
        return None;
    }
    let mut depth = 1;
    let mut cursor = index + 2;
    while cursor < bytes.len() {
        if bytes.get(cursor..cursor + 2) == Some(b"/*") {
            depth += 1;
            cursor += 2;
        } else if bytes.get(cursor..cursor + 2) == Some(b"*/") {
            depth -= 1;
            cursor += 2;
            if depth == 0 {
                return Some(Some(cursor));
            }
        } else {
            cursor += 1;
        }
    }
    Some(None)
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
