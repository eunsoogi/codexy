#![allow(dead_code)]

use std::collections::BTreeSet;

pub(crate) struct Prompt {
    display_name: String,
    default_prompt: String,
    allow_implicit_invocation: bool,
}

impl Prompt {
    pub(crate) fn parse(text: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let value: serde_yaml::Value = serde_yaml::from_str(text)?;
        let interface = value
            .get("interface")
            .and_then(serde_yaml::Value::as_mapping)
            .ok_or("prompt interface")?;
        let policy = value
            .get("policy")
            .and_then(serde_yaml::Value::as_mapping)
            .ok_or("prompt policy")?;
        Ok(Self {
            display_name: yaml_string(interface, "display_name")?.to_owned(),
            default_prompt: yaml_string(interface, "default_prompt")?.to_owned(),
            allow_implicit_invocation: policy
                .get(serde_yaml::Value::from("allow_implicit_invocation"))
                .and_then(serde_yaml::Value::as_bool)
                .ok_or("prompt allow_implicit_invocation")?,
        })
    }

    pub(crate) fn display_name(&self) -> &str {
        &self.display_name
    }

    pub(crate) fn default_prompt(&self) -> &str {
        &self.default_prompt
    }

    pub(crate) fn allow_implicit_invocation(&self) -> bool {
        self.allow_implicit_invocation
    }
}

fn yaml_string<'a>(
    mapping: &'a serde_yaml::Mapping,
    key: &str,
) -> Result<&'a str, Box<dyn std::error::Error>> {
    mapping
        .get(serde_yaml::Value::from(key))
        .and_then(serde_yaml::Value::as_str)
        .ok_or_else(|| format!("prompt {key}").into())
}

pub(crate) struct Template {
    slots: BTreeSet<String>,
}

impl Template {
    pub(crate) fn parse(text: &str) -> Self {
        let slots = text
            .lines()
            .filter_map(|line| line.trim().strip_prefix("- "))
            .filter_map(|line| line.strip_suffix(':'))
            .map(str::to_owned)
            .collect();
        Self { slots }
    }

    pub(crate) fn assert_slots(&self, rule_id: &str, required: &[&str]) {
        let missing: Vec<_> = required
            .iter()
            .filter(|slot| !self.slots.contains(**slot))
            .collect();
        assert!(
            missing.is_empty(),
            "structured contract {rule_id} is missing slots {missing:?}"
        );
    }
}

pub(crate) struct JsonShape {
    value: serde_json::Value,
}

impl JsonShape {
    pub(crate) fn parse(text: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(text).map(|value| Self { value })
    }

    pub(crate) fn assert_bool(&self, rule_id: &str, path: &str, expected: bool) {
        assert_eq!(
            self.value
                .pointer(path)
                .and_then(serde_json::Value::as_bool),
            Some(expected),
            "structured contract {rule_id} has wrong boolean at {path}"
        );
    }

    pub(crate) fn assert_paths(&self, rule_id: &str, required: &[&str]) {
        let missing: Vec<_> = required
            .iter()
            .filter(|path| self.value.pointer(path).is_none())
            .collect();
        assert!(
            missing.is_empty(),
            "structured contract {rule_id} is missing paths {missing:?}"
        );
    }

    pub(crate) fn assert_absent_paths(&self, rule_id: &str, forbidden: &[&str]) {
        let present: Vec<_> = forbidden
            .iter()
            .filter(|path| self.value.pointer(path).is_some())
            .collect();
        assert!(
            present.is_empty(),
            "structured contract {rule_id} has forbidden paths {present:?}"
        );
    }
}
