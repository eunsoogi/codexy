use std::path::Path;

pub(super) fn source_kind(path: &Path, text: &str) -> Option<&'static str> {
    path.extension()
        .and_then(|extension| extension.to_str())
        .and_then(extension_kind)
        .or_else(|| script_shebang_kind(path, text))
}

pub(super) fn is_extensionless_script(path: &Path) -> bool {
    path.starts_with("scripts/") && path.extension().is_none()
}

pub(super) fn portable_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

pub(super) fn visible_code(kind: &str, line: &str) -> String {
    let slash_comments = matches!(kind, "rs" | "js" | "ts" | "tsx" | "jsx");
    let hash_comments = matches!(kind, "py" | "sh" | "ps1" | "yml" | "yaml");
    let mut visible = String::new();
    let mut characters = line.chars().peekable();
    let mut quote = None;
    while let Some(character) = characters.next() {
        if let Some(delimiter) = quote {
            if character == '\\' {
                characters.next();
            } else if character == delimiter {
                quote = None;
            }
        } else if matches!(character, '\'' | '"') {
            quote = Some(character);
        } else if (hash_comments && character == '#')
            || (slash_comments && character == '/' && characters.peek() == Some(&'/'))
        {
            break;
        } else {
            visible.push(character);
        }
    }
    visible
}

fn extension_kind(extension: &str) -> Option<&'static str> {
    match extension {
        "rs" => Some("rs"),
        "py" => Some("py"),
        "sh" => Some("sh"),
        "ps1" => Some("ps1"),
        "js" => Some("js"),
        "ts" => Some("ts"),
        "tsx" => Some("tsx"),
        "jsx" => Some("jsx"),
        "md" => Some("md"),
        "json" => Some("json"),
        "toml" => Some("toml"),
        "yml" => Some("yml"),
        "yaml" => Some("yaml"),
        _ => None,
    }
}

fn script_shebang_kind(path: &Path, text: &str) -> Option<&'static str> {
    is_extensionless_script(path)
        .then_some(text.lines().next()?)
        .and_then(|line| line.strip_prefix("#!"))
        .and_then(|interpreter| {
            interpreter.contains("python").then_some("py").or_else(|| {
                (interpreter.contains("sh") || interpreter.contains("bash")).then_some("sh")
            })
        })
}
