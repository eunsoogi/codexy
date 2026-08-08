use super::{matching_delimiter, skip_non_code};

#[derive(Clone, Copy)]
pub(super) enum ItemBoundary {
    Semicolon,
    BodyOrSemicolon,
}

pub(super) fn item_boundary(token: &str) -> Option<ItemBoundary> {
    match token {
        "use" | "type" | "const" | "static" => Some(ItemBoundary::Semicolon),
        "trait" | "impl" | "fn" | "struct" | "enum" | "union" | "extern" | "macro_rules" => {
            Some(ItemBoundary::BodyOrSemicolon)
        }
        _ => None,
    }
}

pub(super) fn skip_non_module_item(
    bytes: &[u8],
    mut index: usize,
    boundary: ItemBoundary,
) -> Option<usize> {
    let mut angle_depth = 0usize;
    while index < bytes.len() {
        if let Some(next) = skip_non_code(bytes, index)? {
            index = next;
            continue;
        }
        match bytes[index] {
            b';' => return Some(index + 1),
            b'{' => {
                index = matching_delimiter(bytes, index)? + 1;
                if angle_depth == 0 && matches!(boundary, ItemBoundary::BodyOrSemicolon) {
                    return Some(index);
                }
            }
            b'(' | b'[' => index = matching_delimiter(bytes, index)? + 1,
            b'<' => {
                angle_depth += 1;
                index += 1;
            }
            b'>' if angle_depth > 0 => {
                angle_depth -= 1;
                index += 1;
            }
            b'}' | b')' | b']' => return None,
            _ => index += 1,
        }
    }
    None
}
