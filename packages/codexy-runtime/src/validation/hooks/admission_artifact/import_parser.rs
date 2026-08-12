use anyhow::{Result, bail};

pub(super) fn imports(path: &str, source: &str) -> Result<Vec<String>> {
    if source.lines().map(str::trim_start).any(|line| {
        line.starts_with("importlib.")
            || line.starts_with("__import__(")
            || line.starts_with("exec(")
    }) {
        bail!("packaged admission runtime rejects dynamic imports: {path}");
    }
    let mut result = Vec::new();
    for line in source
        .lines()
        .map(|line| line.split('#').next().unwrap_or("").trim())
    {
        if let Some(rest) = line.strip_prefix("import ") {
            for module in modules(rest, path)? {
                if module == "codexy_policy" {
                    bail!("packaged admission runtime rejects ambiguous policy import in {path}");
                }
                if let Some(module) = module.strip_prefix("codexy_policy.") {
                    result.push(policy_path(module, path)?);
                }
            }
        } else if let Some((module, values)) = line
            .strip_prefix("from ")
            .and_then(|line| line.split_once(" import "))
        {
            if module == "codexy_policy" {
                for value in names(values, path)? {
                    result.push(policy_path(&value, path)?);
                }
            } else if let Some(module) = module.strip_prefix("codexy_policy.") {
                result.push(policy_path(module, path)?);
            } else if let Some(module) = module.strip_prefix('.') {
                if module.is_empty() {
                    for value in names(values, path)? {
                        result.push(policy_path(&value, path)?);
                    }
                } else {
                    result.push(policy_path(module, path)?);
                }
            }
        }
    }
    if path.starts_with("codexy_policy/") && path != "codexy_policy/__init__.py" {
        result.push("codexy_policy/__init__.py".to_owned());
    }
    Ok(result)
}

fn names(values: &str, path: &str) -> Result<Vec<String>> {
    values
        .split(',')
        .map(|value| {
            let words = value.split_whitespace().collect::<Vec<_>>();
            match words.as_slice() {
                [name] if *name != "*" && identifier(name) => Ok((*name).to_owned()),
                [name, "as", alias] if identifier(name) && identifier(alias) => {
                    Ok((*name).to_owned())
                }
                _ => bail!("packaged admission runtime rejects ambiguous policy import in {path}"),
            }
        })
        .collect()
}

fn modules(values: &str, path: &str) -> Result<Vec<String>> {
    values
        .split(',')
        .map(|value| {
            let words = value.split_whitespace().collect::<Vec<_>>();
            match words.as_slice() {
                [module] if module_path(module) => Ok((*module).to_owned()),
                [module, "as", alias] if module_path(module) && identifier(alias) => {
                    Ok((*module).to_owned())
                }
                _ => bail!("packaged admission runtime rejects ambiguous policy import in {path}"),
            }
        })
        .collect()
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

fn module_path(value: &str) -> bool {
    value.split('.').all(identifier)
}

#[cfg(test)]
mod tests {
    use super::imports;

    #[test]
    fn imports_track_static_policy_forms_without_tracking_neutral_imports() {
        let imports = imports(
            "codexy_policy/shell_destructive.py",
            "import codexy_policy.shell_github_policy\n\
             import codexy_policy.shell_github as github\n\
             from codexy_policy import shell_github_opaque\n\
             from codexy_policy.shell_github_policy import forbidden\n\
             from .shell_github import evaluate\n\
             from dataclasses import dataclass\n\
             from typing import Any\n",
        )
        .expect("static imports");
        assert_eq!(
            imports,
            vec![
                "codexy_policy/shell_github_policy.py",
                "codexy_policy/shell_github.py",
                "codexy_policy/shell_github_opaque.py",
                "codexy_policy/shell_github_policy.py",
                "codexy_policy/shell_github.py",
                "codexy_policy/__init__.py",
            ]
        );
    }

    #[test]
    fn imports_reject_ambiguous_policy_package_imports() {
        assert!(
            imports(
                "codexy_policy/shell_destructive.py",
                "from codexy_policy import *\n"
            )
            .is_err()
        );
        assert!(
            imports(
                "codexy_policy/shell_destructive.py",
                "import codexy_policy\n"
            )
            .is_err()
        );
    }
}
