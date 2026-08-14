use std::path::Path;

mod javascript;
mod javascript_expression;
mod javascript_template;
mod markdown;
mod powershell;
mod python;
mod rust;
mod shell;
mod shell_heredoc;

#[derive(Clone, Copy, Eq, PartialEq)]
pub(super) enum Language {
    Rust,
    Python,
    Shell,
    PowerShell,
    JavaScript,
    Markdown,
    Json,
    Toml,
    Yaml,
    Other,
}

pub(super) fn language(path: &Path, text: &str) -> Language {
    match path.extension().and_then(|extension| extension.to_str()) {
        Some("rs") => Language::Rust,
        Some("py") => Language::Python,
        Some("sh") => Language::Shell,
        Some("ps1") => Language::PowerShell,
        Some("js" | "ts" | "tsx" | "jsx") => Language::JavaScript,
        Some("md") => Language::Markdown,
        Some("json") => Language::Json,
        Some("toml") => Language::Toml,
        Some("yml" | "yaml") => Language::Yaml,
        None => shebang_language(path, text),
        _ => Language::Other,
    }
}

pub(super) fn visible_lines(language: Language, text: &str) -> Vec<Option<String>> {
    match language {
        Language::Rust => rust::lines(text),
        Language::Python => python::lines(text),
        Language::Shell => shell::lines(text),
        Language::PowerShell => powershell::lines(text),
        Language::JavaScript => javascript::lines(text),
        Language::Markdown => markdown::lines(text),
        _ => text.lines().map(|line| Some(line.to_owned())).collect(),
    }
}

fn shebang_language(path: &Path, text: &str) -> Language {
    if !path.starts_with("scripts") {
        return Language::Other;
    }
    let Some(shebang) = text.lines().next().filter(|line| line.starts_with("#!")) else {
        return Language::Other;
    };
    if shebang.contains("python") {
        Language::Python
    } else if shebang.contains("sh") {
        Language::Shell
    } else {
        Language::Other
    }
}
