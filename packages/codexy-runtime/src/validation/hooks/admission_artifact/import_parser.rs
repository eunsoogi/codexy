use anyhow::{Result, bail};

mod lexical;
use lexical::{Token, dynamic, symbol, tokens, word};

pub(super) fn imports(path: &str, source: &str) -> Result<Vec<String>> {
    let tokens = tokens(source);
    if dynamic(&tokens) {
        bail!("packaged admission runtime rejects dynamic imports: {path}");
    }
    let mut result = Vec::new();
    let mut index = 0;
    while index < tokens.len() {
        if word(&tokens, index) == Some("import") {
            let (modules, next) = modules(&tokens, index + 1, path)?;
            for module in modules {
                if module == "codexy_policy" {
                    bail!("packaged admission runtime rejects ambiguous policy import in {path}");
                }
                if let Some(module) = module.strip_prefix("codexy_policy.") {
                    result.push(policy_path(module, path)?);
                }
            }
            index = next;
        } else if word(&tokens, index) == Some("from") {
            index = from_import(&tokens, index + 1, path, &mut result)?;
        } else {
            index += 1;
        }
    }
    if path.starts_with("codexy_policy/") && path != "codexy_policy/__init__.py" {
        result.push("codexy_policy/__init__.py".to_owned());
    }
    Ok(result)
}

fn from_import(
    tokens: &[Token],
    mut index: usize,
    path: &str,
    result: &mut Vec<String>,
) -> Result<usize> {
    let relative = symbol(tokens, index, '.');
    if relative {
        index += 1;
        if symbol(tokens, index, '.') {
            bail!("packaged admission runtime rejects ambiguous relative import in {path}");
        }
    }
    let (module, next) = if relative && word(tokens, index) == Some("import") {
        (None, index)
    } else {
        module(tokens, index)
    };
    index = next;
    if word(tokens, index) != Some("import") {
        return Ok(index.max(1));
    }
    match (relative, module.as_deref()) {
        (_, Some("codexy_policy")) => {
            let (names, next) = names(tokens, index + 1, path)?;
            for name in names {
                result.push(policy_path(&name, path)?);
            }
            Ok(next)
        }
        (_, Some(module)) if module.starts_with("codexy_policy.") => {
            result.push(policy_path(
                module.trim_start_matches("codexy_policy."),
                path,
            )?);
            Ok(index + 1)
        }
        (true, Some(module)) => {
            result.push(policy_path(module, path)?);
            Ok(index + 1)
        }
        (true, None) => {
            let (names, next) = names(tokens, index + 1, path)?;
            for name in names {
                result.push(policy_path(&name, path)?);
            }
            Ok(next)
        }
        _ => Ok(index + 1),
    }
}

fn modules(tokens: &[Token], mut index: usize, path: &str) -> Result<(Vec<String>, usize)> {
    let mut result = Vec::new();
    loop {
        let (module, next) = module(tokens, index);
        let Some(module) = module else {
            bail!("packaged admission runtime rejects ambiguous policy import in {path}");
        };
        index = next;
        if word(tokens, index) == Some("as") {
            if word(tokens, index + 1)
                .filter(|value| identifier(value))
                .is_none()
            {
                bail!("packaged admission runtime rejects ambiguous policy import in {path}");
            }
            index += 2;
        }
        result.push(module);
        if !symbol(tokens, index, ',') {
            return Ok((result, index));
        }
        index += 1;
    }
}

fn names(tokens: &[Token], mut index: usize, path: &str) -> Result<(Vec<String>, usize)> {
    let mut result = Vec::new();
    loop {
        if symbol(tokens, index, '*') {
            bail!("packaged admission runtime rejects ambiguous policy import in {path}");
        }
        let Some(name) = word(tokens, index).filter(|value| identifier(value)) else {
            bail!("packaged admission runtime rejects ambiguous policy import in {path}");
        };
        result.push(name.to_owned());
        index += 1;
        if word(tokens, index) == Some("as") {
            if word(tokens, index + 1)
                .filter(|value| identifier(value))
                .is_none()
            {
                bail!("packaged admission runtime rejects ambiguous policy import in {path}");
            }
            index += 2;
        }
        if !symbol(tokens, index, ',') {
            return Ok((result, index));
        }
        index += 1;
    }
}

fn module(tokens: &[Token], mut index: usize) -> (Option<String>, usize) {
    let Some(first) = word(tokens, index).filter(|value| identifier(value)) else {
        return (None, index);
    };
    let mut parts = vec![first.to_owned()];
    index += 1;
    while symbol(tokens, index, '.') {
        let Some(part) = word(tokens, index + 1).filter(|value| identifier(value)) else {
            return (None, index);
        };
        parts.push(part.to_owned());
        index += 2;
    }
    (Some(parts.join(".")), index)
}

fn policy_path(module: &str, path: &str) -> Result<String> {
    if module.split('.').all(identifier) {
        Ok(format!("codexy_policy/{}.py", module.replace('.', "/")))
    } else {
        bail!("packaged admission runtime rejects ambiguous policy import in {path}")
    }
}

fn identifier(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|value| value.is_ascii_alphanumeric() || value == '_')
}
#[cfg(test)]
#[path = "import_parser/dynamic_tests.rs"]
mod dynamic_tests;

#[cfg(test)]
#[path = "import_parser/tests.rs"]
mod tests;
