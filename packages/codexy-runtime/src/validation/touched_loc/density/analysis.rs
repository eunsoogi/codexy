use std::path::Path;

pub(super) fn reason(path: &Path, line: &str) -> Option<&'static str> {
    if path.extension().is_some_and(|extension| extension == "md") {
        return (line.matches("MUST ").count() >= 3).then_some("dense mandatory instruction");
    }
    let visible = visible_code(path, line);
    match path.extension().and_then(|extension| extension.to_str()) {
        Some("rs") if visible.contains('{') && statement_count(&visible) >= 3 => {
            Some("dense Rust statements")
        }
        Some("py" | "js" | "ts" | "tsx" | "jsx") if statement_count(&visible) >= 3 => {
            Some("dense executable statements")
        }
        Some("sh" | "ps1") if !shell_predicate(&visible) && command_chain_count(&visible) >= 3 => {
            Some("dense command chain")
        }
        Some("json") if inline_object_fields(&visible, ':') >= 4 => Some("dense JSON object"),
        Some("toml") if inline_object_fields(&visible, '=') >= 4 => Some("dense TOML table"),
        Some("yml" | "yaml") if yaml_flow_fields(&visible) >= 4 => Some("dense YAML flow mapping"),
        Some("yml" | "yaml") if command_chain_count(&visible) >= 3 => {
            Some("dense workflow command chain")
        }
        _ => None,
    }
}

fn visible_code(path: &Path, line: &str) -> String {
    let extension = path.extension().and_then(|item| item.to_str());
    let slash_comment = matches!(extension, Some("rs" | "js" | "ts" | "tsx" | "jsx"));
    let hash_comment = matches!(extension, Some("py" | "sh" | "ps1" | "yml" | "yaml"));
    let mut visible = String::with_capacity(line.len());
    let mut characters = line.chars().peekable();
    let mut quote = None;
    while let Some(character) = characters.next() {
        if let Some(delimiter) = quote {
            if character == '\\' {
                characters.next();
            } else if character == delimiter {
                quote = None;
            }
        } else if matches!(character, '"' | '\'') {
            quote = Some(character);
        } else if (hash_comment && character == '#')
            || (slash_comment && character == '/' && characters.peek() == Some(&'/'))
        {
            break;
        } else {
            visible.push(character);
        }
    }
    visible
}

fn statement_count(line: &str) -> usize {
    line.matches(';').count() + 1
}

fn command_chain_count(line: &str) -> usize {
    line.replace(";;", "")
        .replace("; then", "")
        .replace("; fi", "")
        .matches(';')
        .count()
        + line.matches("&&").count()
        + line.matches("||").count()
        + 1
}

fn shell_predicate(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with("test ")
        || trimmed.starts_with("[ ")
        || trimmed.starts_with("if ")
        || trimmed.starts_with("while ")
        || trimmed.starts_with("for ")
        || trimmed.starts_with("case ")
}

fn inline_object_fields(line: &str, separator: char) -> usize {
    let Some((_, inner)) = line.split_once('{') else {
        return 0;
    };
    let Some((inner, _)) = inner.split_once('}') else {
        return 0;
    };
    inner.matches(separator).count()
}

fn yaml_flow_fields(line: &str) -> usize {
    let Some((prefix, _)) = line.split_once('{') else {
        return 0;
    };
    prefix
        .trim_end()
        .ends_with(':')
        .then(|| inline_object_fields(line, ':'))
        .unwrap_or_default()
}
