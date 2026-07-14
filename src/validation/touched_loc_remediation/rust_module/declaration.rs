use super::attribute::{is_attribute_trivia, path_attribute};
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
    let mut scope = ScopeTracker::default();
    for line in source.lines() {
        let mut line = line.trim();
        let is_outer = scope.is_outer();
        let is_outer_scope = scope.is_outer_scope();
        let outer_remainder = scope.observe_with_outer_remainder(line);
        if !is_outer {
            if outer_attribute_continuation {
                let Some(remainder) = outer_remainder else {
                    continue;
                };
                outer_attribute_continuation = false;
                line = remainder.trim();
                if line.is_empty() {
                    continue;
                }
            } else if is_outer_scope {
                is_attribute_trivia(line, &mut block_comment_depth);
                continue;
            } else {
                attributed_path = None;
                continue;
            }
        }
        if is_attribute_trivia(line, &mut block_comment_depth) {
            continue;
        }
        if let Some(path) = path_attribute(line) {
            attributed_path = Some(path);
            continue;
        }
        let declaration = line
            .strip_prefix("pub(crate) ")
            .or_else(|| line.strip_prefix("pub "))
            .unwrap_or(line);
        let Some(module) = declaration
            .strip_prefix("mod ")
            .and_then(|name| name.strip_suffix(';'))
        else {
            if line.starts_with("#[") {
                outer_attribute_continuation = !scope.is_outer_scope();
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
