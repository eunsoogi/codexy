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
