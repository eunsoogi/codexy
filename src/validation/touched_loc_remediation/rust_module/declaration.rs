use super::attribute::{
    has_cfg_attr_path, is_cfg_disabled, is_outer_attribute, is_path_attribute_start,
    path_attribute_prefix, trivia::is_attribute_trivia,
};
use super::scope::{ScopeTracker, outer_attribute_remainder};
mod inline;

pub(super) struct InlineModule<'a> {
    pub(super) module: &'a str,
    pub(super) body: &'a str,
    pub(super) path: Option<String>,
}

pub(super) fn inline_modules(source: &str) -> Vec<InlineModule<'_>> {
    inline::inline_modules(source)
}

pub(super) struct Declaration {
    pub(super) module: String,
    pub(super) path: Option<String>,
}

pub(super) fn declarations(source: &str) -> Vec<Declaration> {
    let mut declarations = Vec::new();
    let mut attributed_path = None;
    let mut cfg_disabled = false;
    let mut block_comment_depth = 0;
    let mut outer_attribute_continuation = false;
    let mut multiline_outer_attribute: Option<String> = None;
    let mut scope = ScopeTracker::default();
    'lines: for line in source.lines() {
        let mut line = line.trim();
        let mut completed_path_attribute = None;
        let is_outer = scope.is_outer();
        let is_outer_scope = scope.is_outer_scope();
        let outer_remainder = scope.observe_with_outer_remainder(line);
        if !is_outer {
            if outer_attribute_continuation {
                if let Some(attribute) = multiline_outer_attribute.as_mut() {
                    attribute.push('\n');
                    attribute.push_str(line);
                }
                let Some(remainder) = outer_remainder else {
                    continue;
                };
                outer_attribute_continuation = false;
                if let Some(attribute) = multiline_outer_attribute.take() {
                    if let Some((path, remainder)) = path_attribute_prefix(&attribute) {
                        completed_path_attribute = Some((path, remainder.to_owned()));
                    } else if is_path_attribute_start(&attribute) || has_cfg_attr_path(&attribute) {
                        return Vec::new();
                    }
                }
                if completed_path_attribute.is_none() {
                    line = remainder.trim();
                    if line.is_empty() {
                        continue;
                    }
                }
            } else if is_outer_scope {
                let Some(remainder) = outer_remainder else {
                    is_attribute_trivia(line, &mut block_comment_depth);
                    continue;
                };
                block_comment_depth = 0;
                line = remainder.trim();
                if line.is_empty() {
                    continue;
                }
            } else {
                attributed_path = None;
                continue;
            }
        }
        if let Some((path, remainder)) = completed_path_attribute.as_ref() {
            let mut trailing_comment_depth = 0;
            if is_attribute_trivia(remainder, &mut trailing_comment_depth) {
                attributed_path = (trailing_comment_depth == 0).then(|| path.clone());
                continue;
            }
            attributed_path = Some(path.clone());
            line = remainder.trim_start();
        }
        loop {
            if let Some(remainder) = comment_remainder(line, &mut block_comment_depth) {
                line = remainder.trim_start();
                if line.is_empty() {
                    continue 'lines;
                }
            }
            if is_attribute_trivia(line, &mut block_comment_depth) {
                continue 'lines;
            }
            cfg_disabled |= is_cfg_disabled(line);
            if let Some((path, remainder)) = path_attribute_prefix(line) {
                let mut trailing_comment_depth = 0;
                if is_attribute_trivia(remainder, &mut trailing_comment_depth) {
                    attributed_path = (trailing_comment_depth == 0).then_some(path);
                    continue 'lines;
                }
                attributed_path = Some(path);
                line = remainder.trim_start();
            } else if is_path_attribute_start(line) && scope.is_outer_scope() {
                return Vec::new();
            } else if scope.is_outer_scope() && has_cfg_attr_path(line) {
                return Vec::new();
            }
            let Some(remainder) = outer_attribute_remainder(line) else {
                break;
            };
            line = remainder.trim_start();
            if line.is_empty() {
                continue 'lines;
            }
        }
        let Some(module) = declaration_after_visibility(line).and_then(module_declaration) else {
            if is_outer_attribute(line) {
                outer_attribute_continuation = !scope.is_outer_scope();
                if outer_attribute_continuation {
                    multiline_outer_attribute = Some(line.to_owned());
                }
            } else {
                attributed_path = None;
                cfg_disabled = false;
            }
            continue;
        };
        if std::mem::take(&mut cfg_disabled) {
            attributed_path = None;
            continue;
        }
        declarations.push(Declaration {
            module: module.to_owned(),
            path: attributed_path.take(),
        });
    }
    declarations
}

fn comment_remainder<'a>(line: &'a str, depth: &mut usize) -> Option<&'a str> {
    let (comment, mut comment_depth) = if *depth > 0 {
        (line, *depth)
    } else {
        (line.trim_start().strip_prefix("/*")?, 1)
    };
    let bytes = comment.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        match bytes.get(index..index + 2) {
            Some(b"/*") => {
                comment_depth += 1;
                index += 2;
            }
            Some(b"*/") => {
                comment_depth -= 1;
                index += 2;
                if comment_depth == 0 {
                    *depth = 0;
                    return Some(&comment[index..]);
                }
            }
            _ => index += 1,
        }
    }
    *depth = comment_depth;
    None
}

fn declaration_after_visibility(source: &str) -> Option<&str> {
    let Some(remainder) = source.strip_prefix("pub") else {
        return Some(source);
    };
    if remainder
        .as_bytes()
        .first()
        .is_some_and(u8::is_ascii_whitespace)
    {
        return Some(remainder.trim_start());
    }
    let restricted = remainder.strip_prefix('(')?;
    let close = restricted.find(')')?;
    valid_restricted_visibility(&restricted[..close]).then(|| restricted[close + 1..].trim_start())
}

fn valid_restricted_visibility(scope: &str) -> bool {
    matches!(scope, "crate" | "self" | "super")
        || scope.strip_prefix("in ").is_some_and(valid_visibility_path)
}

fn valid_visibility_path(path: &str) -> bool {
    let mut segments = path.split("::");
    matches!(segments.next(), Some("crate" | "self" | "super"))
        && segments
            .all(|segment| identifier(segment).is_some_and(|(_, remainder)| remainder.is_empty()))
}

fn module_declaration(declaration: &str) -> Option<&str> {
    let declaration = module_identifier_remainder(declaration.strip_prefix("mod")?)?;
    let (module, suffix) = identifier(declaration)?;
    let suffix = suffix.trim_start().strip_prefix(';')?;
    let mut comment_depth = 0;
    (is_attribute_trivia(suffix, &mut comment_depth) && comment_depth == 0).then_some(module)
}

fn module_identifier_remainder(mut source: &str) -> Option<&str> {
    let mut separated = false;
    loop {
        let trimmed = source.trim_start();
        separated |= trimmed.len() != source.len();
        source = trimmed;
        let Some(after_comment) = source.strip_prefix("/*") else {
            return separated.then_some(source);
        };
        separated = true;
        let bytes = after_comment.as_bytes();
        let mut depth = 1;
        let mut index = 0;
        while depth > 0 {
            match bytes.get(index..index + 2) {
                Some(b"/*") => depth += 1,
                Some(b"*/") => depth -= 1,
                Some(_) => {
                    index += 1;
                    continue;
                }
                None => return None,
            }
            index += 2;
        }
        source = &after_comment[index..];
    }
}

fn identifier(source: &str) -> Option<(&str, &str)> {
    let bytes = source.as_bytes();
    let mut cursor = 0;
    if bytes.get(..2) == Some(b"r#") {
        cursor = 2;
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
    Some((&source[..cursor], &source[cursor..]))
}
