use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

use super::data::{Mapping, identity as load_identity, manifest as load_manifest};
use super::semantics::{Semantic, destination_values, local_links, normalized_text, trigger};

const ROUTES: [(&str, &str, &str); 6] = [
    ("debugging", "diagnosis.md", "diagnosis"),
    (
        "domain-driven-development",
        "domain-modeling.md",
        "domain-modeling",
    ),
    ("qa", "quality-assurance.md", "quality-assurance"),
    ("refactoring", "refactoring.md", "refactoring"),
    (
        "spec-driven-development",
        "specification.md",
        "specification",
    ),
    (
        "test-driven-development",
        "test-driven-development.md",
        "test-driven-development",
    ),
];

pub(super) fn check(
    plugin_root: &Path,
    sources: &[(String, String)],
    semantics: &[Vec<Semantic>],
) -> Vec<String> {
    let mut errors = Vec::new();
    let references = plugin_root.join("skills/engineering/references");
    let manifest = load_manifest(&references.join("legacy-rule-manifest.json"), &mut errors);
    let Some(manifest) = manifest else {
        return errors;
    };
    if manifest.baseline != "baseline-v1" {
        errors.push("engineering manifest must project baseline-v1".to_owned());
    }
    if manifest.mappings.len() != ROUTES.len() {
        errors.push("engineering manifest must contain exactly six mappings".to_owned());
    }
    let pairs = manifest
        .mappings
        .iter()
        .map(|item| (item.source.as_str(), item.destination.as_str()))
        .collect::<BTreeSet<_>>();
    let expected_pairs = ROUTES
        .iter()
        .map(|(source, destination, _)| (*source, *destination))
        .collect::<BTreeSet<_>>();
    if pairs != expected_pairs {
        errors.push(
            "engineering manifest source/destination pairs must exactly match the baseline routes"
                .to_owned(),
        );
    }
    if pairs.len() != manifest.mappings.len() {
        errors
            .push("engineering manifest source and destination mappings must be unique".to_owned());
    }
    let skill_path = plugin_root.join("skills/engineering/SKILL.md");
    let skill = std::fs::read_to_string(&skill_path).unwrap_or_default();
    errors.extend(local_links(&skill, &skill_path, plugin_root));
    for ((source, text), source_semantics) in sources.iter().zip(semantics) {
        let Some((_, destination, anchor)) = ROUTES.iter().find(|(route, _, _)| route == source)
        else {
            continue;
        };
        let Some(mapping) = manifest.mappings.iter().find(|item| item.source == *source) else {
            errors.push(format!(
                "engineering manifest omits baseline source {source}"
            ));
            continue;
        };
        check_mapping(
            mapping,
            source,
            destination,
            anchor,
            &references,
            &skill,
            source_semantics,
            text,
            plugin_root,
            &mut errors,
        );
    }
    for source in sources.iter().map(|(name, _)| name) {
        let legacy = plugin_root.join("skills").join(source);
        if legacy.join("SKILL.md").is_file() || legacy.join("agents/openai.yaml").is_file() {
            errors.push(format!("legacy engineering bundle remains: {source}"));
        }
    }
    errors
}

#[allow(clippy::too_many_arguments)]
fn check_mapping(
    mapping: &Mapping,
    source: &str,
    destination: &str,
    anchor: &str,
    references: &Path,
    skill: &str,
    source_semantics: &[Semantic],
    source_text: &str,
    plugin_root: &Path,
    errors: &mut Vec<String>,
) {
    if mapping.destination != destination || mapping.entrypoint != format!("SKILL.md#{anchor}") {
        errors.push(format!(
            "engineering mapping for {source} has a stale destination or entrypoint"
        ));
    }
    let expected_trigger = match trigger(source_text) {
        Ok(value) => value,
        Err(error) => {
            errors.push(error);
            return;
        }
    };
    let normalized_skill = normalized_text(skill, &plugin_root.join("skills/engineering/SKILL.md"));
    let normalized_trigger = normalized_text(
        &expected_trigger,
        &plugin_root.join("skills/engineering/SKILL.md"),
    );
    let target = format!("references/{destination}");
    let route = route_clause(skill, &target);
    if normalized_skill.matches(&normalized_trigger).count() != 1
        || skill.matches(&target).count() != 1
        || !route.contains("MUST use [")
        || !normalized_text(route, &plugin_root.join("skills/engineering/SKILL.md"))
            .contains(&normalized_trigger)
    {
        errors.push(format!(
            "engineering route for {source} must expose one exact trigger and destination link"
        ));
    }
    let identity_path = references.join(&mapping.identity_file);
    let identity = load_identity(&identity_path, errors);
    let Some(identity) = identity else {
        return;
    };
    if identity.source != source {
        errors.push(format!("identity projection source differs for {source}"));
    }
    let expected = source_semantics
        .iter()
        .map(|item| item.id.clone())
        .collect::<BTreeSet<_>>();
    let actual = identity.identities.iter().cloned().collect::<BTreeSet<_>>();
    if actual.len() != identity.identities.len() {
        errors.push(format!(
            "identity projection duplicates a rule for {source}"
        ));
    }
    if actual != expected {
        let missing = expected
            .difference(&actual)
            .cloned()
            .collect::<Vec<_>>()
            .join(",");
        let unknown = actual
            .difference(&expected)
            .cloned()
            .collect::<Vec<_>>()
            .join(",");
        errors.push(format!("identity projection must exactly cover baseline identities for {source}; missing={missing}; unknown={unknown}"));
    }
    let path = references.join(destination);
    let destination_text = std::fs::read_to_string(&path).unwrap_or_default();
    errors.extend(local_links(&destination_text, &path, plugin_root));
    let expected_values = counts(
        source_semantics
            .iter()
            .filter(|item| item.id.contains(":semantic:"))
            .map(|item| item.value.as_str()),
    );
    let destination_values = destination_values(&destination_text, &path);
    let actual_values = counts(destination_values.iter().map(String::as_str));
    if expected_values != actual_values {
        errors.push(format!(
            "engineering destination equivalence differs for {source}"
        ));
    }
}

fn route_clause<'a>(skill: &'a str, target: &str) -> &'a str {
    let Some(link) = skill.find(target) else {
        return "";
    };
    let start = skill[..link].rfind("\n## ").map_or(0, |index| index + 1);
    let end = skill[link..]
        .find("\n## ")
        .map_or(skill.len(), |index| link + index);
    &skill[start..end]
}

fn counts<'a>(values: impl Iterator<Item = &'a str>) -> BTreeMap<&'a str, usize> {
    let mut counts = BTreeMap::new();
    for value in values {
        *counts.entry(value).or_default() += 1;
    }
    counts
}
