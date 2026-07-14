use super::attribute::{is_attribute_trivia, path_attribute_prefix};
use super::scope::ScopeTracker;

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
    let mut block_comment_depth = 0;
    let mut outer_attribute_continuation = false;
    let mut multiline_path_attribute: Option<String> = None;
    let mut scope = ScopeTracker::default();
    for line in source.lines() {
        let mut line = line.trim();
        let mut completed_path_attribute = None;
        let is_outer = scope.is_outer();
        let is_outer_scope = scope.is_outer_scope();
        let outer_remainder = scope.observe_with_outer_remainder(line);
        if !is_outer {
            if outer_attribute_continuation {
                if let Some(attribute) = multiline_path_attribute.as_mut() {
                    attribute.push('\n');
                    attribute.push_str(line);
                }
                let Some(remainder) = outer_remainder else {
                    continue;
                };
                outer_attribute_continuation = false;
                if let Some(attribute) = multiline_path_attribute.take() {
                    completed_path_attribute = path_attribute_prefix(&attribute)
                        .map(|(path, remainder)| (path, remainder.to_owned()));
                    if completed_path_attribute.is_none() {
                        attributed_path = None;
                    }
                }
                if completed_path_attribute.is_none() {
                    line = remainder.trim();
                    if line.is_empty() {
                        continue;
                    }
                }
            } else if is_outer_scope {
                is_attribute_trivia(line, &mut block_comment_depth);
                continue;
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
        } else {
            if is_attribute_trivia(line, &mut block_comment_depth) {
                continue;
            }
            if let Some((path, remainder)) = path_attribute_prefix(line) {
                let mut trailing_comment_depth = 0;
                if is_attribute_trivia(remainder, &mut trailing_comment_depth) {
                    attributed_path = (trailing_comment_depth == 0).then_some(path);
                    continue;
                }
                attributed_path = Some(path);
                line = remainder.trim_start();
            }
        }
        let declaration = line
            .strip_prefix("pub(crate) ")
            .or_else(|| line.strip_prefix("pub "))
            .unwrap_or(line);
        let Some(module) = module_declaration(declaration) else {
            if line.starts_with("#[") {
                outer_attribute_continuation = !scope.is_outer_scope();
                if outer_attribute_continuation && line.starts_with("#[path") {
                    multiline_path_attribute = Some(line.to_owned());
                }
            } else {
                attributed_path = None;
            }
            continue;
        };
        declarations.push(Declaration {
            module: module.to_owned(),
            path: attributed_path.take(),
        });
    }
    declarations
}

fn module_declaration(declaration: &str) -> Option<&str> {
    let declaration = declaration.strip_prefix("mod ")?;
    let (module, suffix) = identifier(declaration)?;
    let suffix = suffix.trim_start().strip_prefix(';')?;
    let mut comment_depth = 0;
    (is_attribute_trivia(suffix, &mut comment_depth) && comment_depth == 0).then_some(module)
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
