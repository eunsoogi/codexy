use std::path::Path;

use toml::Value;

use crate::paths::display_relative;

const ALLOWED_SKILLS_CONFIG_FIELDS: &[&str] = &["enabled", "path"];

pub(super) fn check_skills_config(path: &Path, value: Option<&Value>, errors: &mut Vec<String>) {
    let Some(value) = value else {
        return;
    };
    let Value::Table(skills) = value else {
        errors.push(format!("{} skills must be a table", display_relative(path)));
        return;
    };

    for (key, value) in skills {
        if key.as_str() != "config" {
            errors.push(format!(
                "{} skills.{key} is not part of the supported Codex custom-agent file schema",
                display_relative(path)
            ));
            continue;
        }
        check_config_entries(path, value, errors);
    }
}

fn check_config_entries(path: &Path, value: &Value, errors: &mut Vec<String>) {
    let Value::Array(entries) = value else {
        errors.push(format!(
            "{} skills.config must be an array of tables",
            display_relative(path)
        ));
        return;
    };

    for entry in entries {
        let Value::Table(fields) = entry else {
            errors.push(format!(
                "{} skills.config must contain only tables",
                display_relative(path)
            ));
            continue;
        };
        for key in fields.keys() {
            if !ALLOWED_SKILLS_CONFIG_FIELDS.contains(&key.as_str()) {
                errors.push(format!(
                    "{} skills.config.{key} is not part of the supported Codex custom-agent file schema",
                    display_relative(path)
                ));
            }
        }
        if fields.get("path").is_some_and(|path| !path.is_str()) {
            errors.push(format!(
                "{} skills.config.path must be a string",
                display_relative(path)
            ));
        }
        if fields
            .get("enabled")
            .is_some_and(|enabled| !enabled.is_bool())
        {
            errors.push(format!(
                "{} skills.config.enabled must be a boolean",
                display_relative(path)
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::check_skills_config;

    #[test]
    fn non_table_skills_matrix_preserves_exact_diagnostics() {
        let path = Path::new("agents/codexy-pathfinder.toml");
        for fragment in ["\nskills = []\n", "\nskills = \"disabled\"\n"] {
            let parsed = toml::from_str::<toml::Table>(fragment).expect("valid TOML fragment");
            let mut errors = Vec::new();
            check_skills_config(path, parsed.get("skills"), &mut errors);
            assert_eq!(
                errors,
                vec!["agents/codexy-pathfinder.toml skills must be a table".to_owned()],
                "input: {fragment:?}"
            );
        }
    }
}
