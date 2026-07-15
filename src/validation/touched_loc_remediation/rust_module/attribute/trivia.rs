pub(crate) fn is_attribute_trivia(line: &str, block_comment_depth: &mut usize) -> bool {
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

pub(super) fn attribute_name_suffix(mut suffix: &str) -> Option<&str> {
    loop {
        suffix = suffix.trim_start();
        let Some(comment) = suffix.strip_prefix("/*") else {
            return Some(suffix);
        };
        suffix = &comment[block_comment_end(comment)?..];
    }
}

fn block_comment_end(comment: &str) -> Option<usize> {
    let bytes = comment.as_bytes();
    let mut depth = 1;
    let mut index = 0;
    while index < bytes.len() {
        match bytes.get(index..index + 2) {
            Some(b"/*") => depth += 1,
            Some(b"*/") => {
                depth -= 1;
                if depth == 0 {
                    return Some(index + 2);
                }
            }
            _ => {
                index += 1;
                continue;
            }
        }
        index += 2;
    }
    None
}
