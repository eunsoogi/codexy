use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result};
use serde_json::Value;

use crate::lsp::config::Server;

const WORKSPACE_MARKERS: &[&str] = &[
    ".git",
    "package.json",
    "pyproject.toml",
    "Cargo.toml",
    "go.mod",
    "deno.json",
    "tsconfig.json",
    "jsconfig.json",
];

pub(super) fn resolve_path(file_path: &str, root: Option<&str>) -> Result<String> {
    let path = Path::new(file_path);
    if path.is_absolute() {
        return Ok(path.display().to_string());
    }
    let root = root.context("root or workspaceRoot is required for a relative path")?;
    Ok(resolve_root(root)?.join(path).display().to_string())
}

pub(super) fn workspace_root_for_file(file_path: &str) -> PathBuf {
    let path = Path::new(file_path);
    let initial = fs::metadata(path)
        .ok()
        .and_then(|meta| meta.is_dir().then(|| path.to_path_buf()))
        .unwrap_or_else(|| path.parent().unwrap_or(path).to_path_buf());
    let mut directory = initial.clone();
    loop {
        if WORKSPACE_MARKERS
            .iter()
            .any(|marker| directory.join(marker).exists())
        {
            return directory;
        }
        let Some(parent) = directory.parent() else {
            return initial;
        };
        if parent == directory {
            return initial;
        }
        directory = parent.to_path_buf();
    }
}

pub(super) fn root_from_args(args: &Value) -> Option<&str> {
    args.get("root")
        .or_else(|| args.get("workspaceRoot"))
        .and_then(Value::as_str)
}

pub(super) fn match_path_from_args(path: &str, args: &Value) -> Result<String> {
    root_from_args(args).map_or_else(
        || Ok(path.to_owned()),
        |root| resolve_path(path, Some(root)),
    )
}

pub(super) fn normalize_ext(file_path: &str) -> String {
    let path = Path::new(file_path);
    if path.file_name().and_then(|item| item.to_str()) == Some("Dockerfile") {
        return "Dockerfile".to_owned();
    }
    path.extension()
        .and_then(|item| item.to_str())
        .map_or(String::new(), |extension| format!(".{extension}"))
}

pub(super) fn language_for_path(file_path: &str, server: &Server) -> String {
    let extension = normalize_ext(file_path);
    let language = match extension.as_str() {
        ".js" | ".mjs" | ".cjs" => Some("javascript"),
        ".jsx" => Some("javascriptreact"),
        ".ts" | ".mts" | ".cts" => Some("typescript"),
        ".tsx" => Some("typescriptreact"),
        ".json" => Some("json"),
        ".jsonc" => Some("jsonc"),
        ".scss" => Some("scss"),
        ".less" => Some("less"),
        ".py" | ".pyi" => Some("python"),
        ".rs" => Some("rust"),
        ".go" => Some("go"),
        ".md" | ".markdown" => Some("markdown"),
        ".yaml" | ".yml" => Some("yaml"),
        ".toml" => Some("toml"),
        ".sh" | ".bash" | ".zsh" | ".ksh" => Some("shellscript"),
        ".c" | ".h" => Some("c"),
        ".cc" | ".cpp" | ".cxx" | ".hh" | ".hpp" | ".hxx" => Some("cpp"),
        ".cs" | ".csx" => Some("csharp"),
        ".fs" | ".fsi" | ".fsx" => Some("fsharp"),
        "Dockerfile" => Some("dockerfile"),
        _ => None,
    };
    if let Some(language) = language {
        return language.to_owned();
    }
    let label = server.language.as_deref().unwrap_or(&server.id);
    language_for_label(label)
}

pub(super) fn to_file_uri(file_path: &str) -> Result<String> {
    let path = Path::new(file_path);
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    Ok(format!(
        "file://{}",
        percent_encode_path(&absolute.display().to_string())
    ))
}

pub(super) fn resolve_root(root: &str) -> Result<PathBuf> {
    let path = Path::new(root);
    if path.is_absolute() {
        return Ok(path.to_path_buf());
    }
    Ok(std::env::current_dir()?.join(path))
}

fn language_for_label(label: &str) -> String {
    match label {
        "Astro" => "astro",
        "C/C++" => "cpp",
        "C#" => "csharp",
        "Clojure" => "clojure",
        "Dockerfile" => "dockerfile",
        "Elixir" => "elixir",
        "F#" => "fsharp",
        "Gleam" => "gleam",
        "Haskell" => "haskell",
        "Java" => "java",
        "Julia" => "julia",
        "Kotlin" => "kotlin",
        "Lua" => "lua",
        "Nix" => "nix",
        "OCaml" => "ocaml",
        "PHP" => "php",
        "Prisma" => "prisma",
        "Ruby" => "ruby",
        "Shell" => "shellscript",
        "Swift" => "swift",
        "Svelte" => "svelte",
        "Terraform" => "terraform",
        "TypeScript and JavaScript" => "javascript",
        "LaTeX" => "latex",
        "Typst" => "typst",
        "Vue" => "vue",
        "Zig" => "zig",
        other => return other.to_ascii_lowercase().replace(['#', '/'], ""),
    }
    .to_owned()
}

fn percent_encode_path(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'/' | b'-' | b'_' | b'.' | b'~' => {
                output.push(char::from(byte));
            }
            _ => {
                let _ = write!(output, "%{byte:02X}");
            }
        }
    }
    output
}
