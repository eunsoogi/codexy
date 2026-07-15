use super::super::attribute::{
    has_cfg_attr_path, is_cfg_disabled, is_path_attribute_start, path_attribute,
};
use super::{InlineModule, valid_visibility_path};
mod item;
mod literal;
use item::{item_boundary, skip_non_module_item};
pub(super) fn inline_modules(source: &str) -> Vec<InlineModule<'_>> {
    parse(source).unwrap_or_default()
}

fn parse(source: &str) -> Option<Vec<InlineModule<'_>>> {
    let bytes = source.as_bytes();
    let mut modules = Vec::new();
    let mut index = 0;
    let mut delimiters = Vec::new();
    let mut attributed_path = None;
    let mut cfg_disabled = false;
    let mut invalid_prefix = false;
    while index < bytes.len() {
        if let Some(next) = skip_non_code(bytes, index)? {
            index = next;
            continue;
        }
        if delimiters.is_empty() && bytes.get(index) == Some(&b'#') {
            let attribute_start = skip_trivia(bytes, index + 1)?;
            if bytes.get(attribute_start) == Some(&b'[') {
                let end = matching_delimiter(bytes, attribute_start)?;
                cfg_disabled |= is_cfg_disabled(&source[index..=end]);
                if let Some(path) = path_attribute(&source[index..=end]) {
                    attributed_path = Some(path);
                } else if is_path_attribute_start(&source[index..=end])
                    || has_cfg_attr_path(&source[index..=end])
                {
                    return None;
                }
                index = end + 1;
                continue;
            }
        }
        match bytes[index] {
            b'{' | b'(' | b'[' => {
                if delimiters.is_empty() {
                    attributed_path = None;
                    cfg_disabled = false;
                    invalid_prefix = false;
                }
                delimiters.push(bytes[index]);
                index += 1;
            }
            b'}' | b')' | b']' => {
                close_delimiter(&mut delimiters, bytes[index])?;
                index += 1;
            }
            b';' if delimiters.is_empty() => {
                attributed_path = None;
                cfg_disabled = false;
                invalid_prefix = false;
                index += 1;
            }
            _ if delimiters.is_empty() => {
                let Some((token, next)) = identifier(source, index) else {
                    index += 1;
                    continue;
                };
                index = next;
                if token == "pub" {
                    let cursor = skip_trivia(bytes, index)?;
                    if bytes.get(cursor) == Some(&b'(') {
                        let end = matching_delimiter(bytes, cursor)?;
                        if restricted_visibility(&source[cursor + 1..end]) {
                            index = end + 1;
                        } else {
                            return None;
                        }
                    }
                    continue;
                }
                if invalid_prefix && token == "mod" {
                    return None;
                }
                if token != "mod" {
                    if attributed_path.take().is_some() || cfg_disabled {
                        index = skip_non_module_item(bytes, index, item_boundary(token)?)?;
                    }
                    cfg_disabled = false;
                    invalid_prefix = bytes.get(index) != Some(&b'!');
                    continue;
                }
                let cursor = skip_trivia(bytes, index)?;
                let Some((module, cursor)) = identifier(source, cursor) else {
                    attributed_path = None;
                    cfg_disabled = false;
                    continue;
                };
                let cursor = skip_trivia(bytes, cursor)?;
                if bytes.get(cursor) != Some(&b'{') {
                    attributed_path = None;
                    cfg_disabled = false;
                    continue;
                }
                let end = matching_delimiter(bytes, cursor)?;
                if !std::mem::take(&mut cfg_disabled) {
                    modules.push(InlineModule {
                        module,
                        body: &source[cursor + 1..end],
                        path: attributed_path.take(),
                    });
                } else {
                    attributed_path = None;
                }
                index = end + 1;
            }
            _ => index += 1,
        }
    }
    delimiters.is_empty().then_some(modules)
}
fn matching_delimiter(bytes: &[u8], start: usize) -> Option<usize> {
    let mut delimiters = vec![*bytes.get(start)?];
    let mut index = start + 1;
    while index < bytes.len() {
        if let Some(next) = skip_non_code(bytes, index)? {
            index = next;
            continue;
        }
        match bytes[index] {
            b'{' | b'(' | b'[' => delimiters.push(bytes[index]),
            b'}' | b')' | b']' => {
                close_delimiter(&mut delimiters, bytes[index])?;
                if delimiters.is_empty() {
                    return Some(index);
                }
            }
            _ => {}
        }
        index += 1;
    }
    None
}
fn close_delimiter(delimiters: &mut Vec<u8>, close: u8) -> Option<()> {
    let expected = match close {
        b'}' => b'{',
        b')' => b'(',
        b']' => b'[',
        _ => return None,
    };
    (delimiters.pop() == Some(expected)).then_some(())
}
fn skip_non_code(bytes: &[u8], index: usize) -> Option<Option<usize>> {
    if bytes.get(index..index + 2) == Some(b"//") {
        return Some(Some(
            bytes[index + 2..]
                .iter()
                .position(|byte| *byte == b'\n')
                .map_or(bytes.len(), |offset| index + offset + 3),
        ));
    }
    if bytes.get(index..index + 2) == Some(b"/*") {
        return Some(Some(skip_block_comment(bytes, index + 2)?));
    }
    literal::skip(bytes, index)
}
fn skip_block_comment(bytes: &[u8], mut index: usize) -> Option<usize> {
    let mut depth = 1;
    while index < bytes.len() {
        if bytes.get(index..index + 2) == Some(b"/*") {
            depth += 1;
            index += 2;
        } else if bytes.get(index..index + 2) == Some(b"*/") {
            depth -= 1;
            index += 2;
            if depth == 0 {
                return Some(index);
            }
        } else {
            index += 1;
        }
    }
    None
}
fn skip_trivia(bytes: &[u8], mut index: usize) -> Option<usize> {
    loop {
        while bytes.get(index).is_some_and(u8::is_ascii_whitespace) {
            index += 1;
        }
        let Some(next) = skip_non_code(bytes, index)? else {
            return Some(index);
        };
        index = next;
    }
}
fn identifier(source: &str, index: usize) -> Option<(&str, usize)> {
    let bytes = source.as_bytes();
    let mut cursor = index;
    if bytes.get(cursor..cursor + 2) == Some(b"r#") {
        cursor += 2;
    }
    let first = *bytes.get(cursor)?;
    (first == b'_' || first.is_ascii_alphabetic() || !first.is_ascii()).then_some(())?;
    cursor += 1;
    while bytes
        .get(cursor)
        .is_some_and(|byte| *byte == b'_' || byte.is_ascii_alphanumeric() || !byte.is_ascii())
    {
        cursor += 1;
    }
    Some((&source[index..cursor], cursor))
}
fn restricted_visibility(source: &str) -> bool {
    let source = source.trim();
    if matches!(source, "crate" | "self" | "super") {
        return true;
    }
    let Some(path) = source.strip_prefix("in") else {
        return false;
    };
    path.as_bytes().first().is_some_and(u8::is_ascii_whitespace)
        && valid_visibility_path(path.trim())
}
