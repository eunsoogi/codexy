use super::spans::Language;

pub(super) fn reason(language: Language, line: &str) -> Option<&'static str> {
    if language == Language::Markdown {
        return markdown_clause_count(line)
            .ge(&3)
            .then_some("dense Markdown clauses");
    }
    let visible = visible_code(language, line);
    match language {
        Language::Rust if visible.contains('{') && statement_count(&visible) >= 3 => {
            Some("dense Rust statements")
        }
        Language::Python if python_inline_suite(&visible) || statement_count(&visible) >= 2 => {
            Some("dense executable statements")
        }
        Language::JavaScript if statement_count(&visible) >= 3 => {
            Some("dense executable statements")
        }
        Language::Shell | Language::PowerShell
            if command_chain_count(shell_body(&visible)) >= 3 =>
        {
            Some("dense command chain")
        }
        Language::Json if inline_object_fields(&visible, ':') >= 4 => Some("dense JSON object"),
        Language::Toml if inline_object_fields(&visible, '=') >= 4 => Some("dense TOML table"),
        Language::Yaml if yaml_flow_fields(&visible) >= 4 => Some("dense YAML flow mapping"),
        Language::Yaml if command_chain_count(&yaml_commands(&visible)) >= 3 => {
            Some("dense workflow command chain")
        }
        _ => None,
    }
}

fn visible_code(language: Language, line: &str) -> String {
    let slash_comment = matches!(language, Language::Rust | Language::JavaScript);
    let hash_comment = matches!(
        language,
        Language::Python | Language::Shell | Language::PowerShell | Language::Yaml
    );
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
        } else if matches!(character, '"') || (character == '\'' && language != Language::Rust) {
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

fn yaml_commands(line: &str) -> String {
    let mut visible = String::new();
    let mut remainder = line;
    while let Some(start) = remainder.find("${{") {
        visible.push_str(&remainder[..start]);
        let Some(end) = remainder[start + 3..].find("}}") else {
            return visible;
        };
        remainder = &remainder[start + end + 5..];
    }
    visible.push_str(remainder);
    visible
}

fn statement_count(line: &str) -> usize {
    let mut count = 0;
    let mut current = String::new();
    let mut parentheses: usize = 0;
    let mut brackets: usize = 0;
    let mut brace_parentheses = Vec::new();
    for character in line.chars() {
        match character {
            '(' => parentheses += 1,
            ')' => parentheses = parentheses.saturating_sub(1),
            '[' => brackets += 1,
            ']' => brackets = brackets.saturating_sub(1),
            '{' => brace_parentheses.push(parentheses),
            '}' => {
                brace_parentheses.pop();
            }
            ';' if brackets == 0
                && (parentheses == 0
                    || brace_parentheses
                        .last()
                        .is_some_and(|depth| *depth == parentheses)) =>
            {
                count += statement_fragment(&current) as usize;
                current.clear();
                continue;
            }
            _ => {}
        }
        current.push(character);
    }
    count + statement_fragment(&current) as usize
}

fn statement_fragment(fragment: &str) -> bool {
    !fragment.trim().trim_matches(['{', '}']).trim().is_empty()
}

fn command_chain_count(line: &str) -> usize {
    let line = line
        .replace(";;", "")
        .replace("; then", "")
        .replace("; fi", "");
    let line = line.trim().trim_end_matches(';');
    line.matches(';').count() + line.matches("&&").count() + line.matches("||").count() + 1
}

fn shell_body(line: &str) -> &str {
    let trimmed = line.trim_start();
    if let Some((_, body)) = trimmed.split_once("|| {") {
        body.strip_suffix('}').unwrap_or(body)
    } else if matches!(
        trimmed.split_whitespace().next(),
        Some("if" | "while" | "until" | "for")
    ) {
        trimmed
            .split_once("; then")
            .or_else(|| trimmed.split_once("; do"))
            .map_or("", |(_, body)| body)
    } else if trimmed.starts_with("case ") {
        trimmed.split_once(')').map_or("", |(_, body)| body)
    } else {
        trimmed
    }
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
