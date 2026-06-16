pub(super) fn code_position_mask(source: &str) -> Vec<bool> {
    let bytes = source.as_bytes();
    let mut mask = vec![true; bytes.len()];
    let mut index = 0usize;
    while index < bytes.len() {
        match bytes[index] {
            b'\'' | b'"' => index = mask_quoted(bytes, &mut mask, index, bytes[index]),
            b'`' => index = mask_template(bytes, &mut mask, index),
            b'/' if bytes.get(index + 1) == Some(&b'/') => {
                index = mask_until_newline(bytes, &mut mask, index);
            }
            b'/' if bytes.get(index + 1) == Some(&b'*') => {
                index = mask_block_comment(bytes, &mut mask, index);
            }
            b'/' if starts_regex_literal(bytes, &mask, index) => {
                index = mask_regex_literal(bytes, &mut mask, index);
            }
            _ => index += 1,
        }
    }
    mask
}

pub(super) fn language_mask(source: &str, extension: &str) -> Vec<bool> {
    let mut mask = code_position_mask(source);
    if matches!(extension, ".py" | ".rb") {
        let bytes = source.as_bytes();
        let mut index = 0usize;
        while index < bytes.len() {
            if mask.get(index).copied().unwrap_or(false) && bytes[index] == b'#' {
                while index < bytes.len() && bytes[index] != b'\n' {
                    if let Some(slot) = mask.get_mut(index) {
                        *slot = false;
                    }
                    index += 1;
                }
            }
            index += 1;
        }
    }
    mask
}

fn mask_quoted(bytes: &[u8], mask: &mut [bool], start: usize, quote: u8) -> usize {
    let mut index = start;
    let mut escaped = false;
    while index < bytes.len() {
        mask[index] = false;
        if escaped {
            escaped = false;
        } else if bytes[index] == b'\\' {
            escaped = true;
        } else if index != start && bytes[index] == quote {
            return index + 1;
        }
        index += 1;
    }
    index
}

fn mask_template(bytes: &[u8], mask: &mut [bool], start: usize) -> usize {
    let mut index = start;
    let mut escaped = false;
    while index < bytes.len() {
        mask[index] = false;
        if escaped {
            escaped = false;
        } else if bytes[index] == b'\\' {
            escaped = true;
        } else if bytes[index] == b'$' && bytes.get(index + 1) == Some(&b'{') {
            if let Some(slot) = mask.get_mut(index + 1) {
                *slot = false;
            }
            index = mask_template_expression(bytes, mask, index + 2);
            continue;
        } else if index != start && bytes[index] == b'`' {
            return index + 1;
        }
        index += 1;
    }
    index
}

fn mask_template_expression(bytes: &[u8], mask: &mut [bool], start: usize) -> usize {
    let mut index = start;
    let mut depth = 1usize;
    while index < bytes.len() {
        match bytes[index] {
            b'\'' | b'"' => index = mask_quoted(bytes, mask, index, bytes[index]),
            b'`' => index = mask_template(bytes, mask, index),
            b'/' if bytes.get(index + 1) == Some(&b'/') => {
                index = mask_until_newline(bytes, mask, index);
            }
            b'/' if bytes.get(index + 1) == Some(&b'*') => {
                index = mask_block_comment(bytes, mask, index);
            }
            b'/' if starts_regex_literal(bytes, mask, index) => {
                index = mask_regex_literal(bytes, mask, index);
            }
            b'{' => {
                depth += 1;
                index += 1;
            }
            b'}' => {
                depth = depth.saturating_sub(1);
                mask[index] = false;
                index += 1;
                if depth == 0 {
                    return index;
                }
            }
            _ => index += 1,
        }
    }
    index
}

fn mask_until_newline(bytes: &[u8], mask: &mut [bool], start: usize) -> usize {
    let mut index = start;
    while index < bytes.len() && bytes[index] != b'\n' {
        mask[index] = false;
        index += 1;
    }
    index
}

fn mask_block_comment(bytes: &[u8], mask: &mut [bool], start: usize) -> usize {
    let mut index = start;
    while index < bytes.len() {
        mask[index] = false;
        if bytes[index] == b'*' && bytes.get(index + 1) == Some(&b'/') {
            if let Some(slot) = mask.get_mut(index + 1) {
                *slot = false;
            }
            return index + 2;
        }
        index += 1;
    }
    index
}

fn starts_regex_literal(bytes: &[u8], mask: &[bool], slash: usize) -> bool {
    if bytes
        .get(slash + 1)
        .is_none_or(|next| matches!(next, b'/' | b'*'))
    {
        return false;
    }
    let mut index = slash;
    while index > 0 {
        index -= 1;
        if !mask.get(index).copied().unwrap_or(false) || bytes[index].is_ascii_whitespace() {
            continue;
        }
        if bytes[index].is_ascii_alphanumeric() || bytes[index] == b'_' || bytes[index] == b'$' {
            return regex_prefix_keyword(bytes, index + 1).is_some_and(is_regex_prefix_keyword);
        }
        return matches!(
            bytes[index],
            b'(' | b'['
                | b'{'
                | b'='
                | b':'
                | b','
                | b';'
                | b'!'
                | b'&'
                | b'|'
                | b'?'
                | b'+'
                | b'-'
                | b'*'
                | b'~'
                | b'^'
                | b'<'
                | b'>'
        );
    }
    true
}

fn regex_prefix_keyword(bytes: &[u8], end: usize) -> Option<&str> {
    let mut start = end;
    while start > 0 {
        let previous = bytes[start - 1];
        if previous.is_ascii_alphanumeric() || previous == b'_' || previous == b'$' {
            start -= 1;
        } else {
            break;
        }
    }
    std::str::from_utf8(&bytes[start..end]).ok()
}

const fn is_regex_prefix_keyword(word: &str) -> bool {
    matches!(
        word.as_bytes(),
        b"return"
            | b"throw"
            | b"case"
            | b"delete"
            | b"void"
            | b"typeof"
            | b"yield"
            | b"await"
            | b"in"
            | b"of"
            | b"instanceof"
    )
}

fn mask_regex_literal(bytes: &[u8], mask: &mut [bool], start: usize) -> usize {
    let mut index = start;
    let mut escaped = false;
    let mut in_class = false;
    while index < bytes.len() {
        mask[index] = false;
        if escaped {
            escaped = false;
        } else if bytes[index] == b'\\' {
            escaped = true;
        } else if bytes[index] == b'[' {
            in_class = true;
        } else if bytes[index] == b']' {
            in_class = false;
        } else if index != start && bytes[index] == b'/' && !in_class {
            index += 1;
            while index < bytes.len() && bytes[index].is_ascii_alphabetic() {
                mask[index] = false;
                index += 1;
            }
            return index;
        }
        index += 1;
    }
    index
}
