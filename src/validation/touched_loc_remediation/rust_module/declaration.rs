use super::attribute::{is_attribute_trivia, path_attribute};
use super::scope::ScopeTracker;

pub(super) struct Declaration {
    pub(super) module: String,
    pub(super) path: Option<String>,
}

pub(super) fn declarations(source: &str) -> Vec<Declaration> {
    let mut declarations = Vec::new();
    let mut attributed_path = None;
    let mut block_comment_depth = 0;
    let mut scope = ScopeTracker::default();
    for line in source.lines() {
        let line = line.trim();
        let is_outer = scope.is_outer();
        let is_outer_scope = scope.is_outer_scope();
        scope.observe(line);
        if !is_outer {
            if is_outer_scope {
                is_attribute_trivia(line, &mut block_comment_depth);
            } else {
                attributed_path = None;
            }
            continue;
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
            if !line.starts_with("#[") {
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
