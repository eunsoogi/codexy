use std::path::Path;

pub(super) fn reason(path: &Path, line: &str) -> Option<&'static str> {
    if path.extension().is_some_and(|extension| extension == "md") {
        return markdown_clause_count(line)
            .ge(&3)
            .then_some("dense Markdown clauses");
    }
    let visible = visible_code(path, line);
    match path.extension().and_then(|extension| extension.to_str()) {
        Some("rs") if visible.contains('{') && statement_count(&visible) >= 3 => {
            Some("dense Rust statements")
        }
        Some("py") if python_inline_suite(&visible) || statement_count(&visible) >= 3 => {
            Some("dense executable statements")
        }
        Some("js" | "ts" | "tsx" | "jsx")
            if !javascript_header(&visible) && statement_count(&visible) >= 3 =>
        {
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

pub(super) fn markdown_nonprose_lines(path: &Path, text: &str) -> Vec<bool> {
    if path.extension().is_none_or(|extension| extension != "md") {
        return vec![false; text.lines().count()];
    }
    let mut fenced = false;
    text.lines()
        .map(|line| {
            let trimmed = line.trim_start();
            if trimmed.starts_with("```") {
                fenced = !fenced;
                true
            } else {
                fenced || trimmed.starts_with('|')
            }
        })
        .collect()
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
        } else if matches!(character, '"') || (character == '\'' && !rust_lifetime(&visible)) {
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

fn markdown_clause_count(line: &str) -> usize {
    line.split(';')
        .filter(|clause| clause.split_whitespace().count() >= 3)
        .count()
}

fn python_inline_suite(line: &str) -> bool {
    line.trim_start()
        .split_once(':')
        .is_some_and(|(header, suite)| {
            (header.starts_with("if ")
                || header.starts_with("for ")
                || header.starts_with("while ")
                || header.starts_with("def ")
                || header.starts_with("class "))
                && suite.trim().contains(';')
        })
}

fn javascript_header(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with("for (") || trimmed.starts_with("for(")
}

fn rust_lifetime(visible: &str) -> bool {
    visible
        .chars()
        .last()
        .is_some_and(|character| character.is_ascii_alphanumeric() || character == '_')
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
