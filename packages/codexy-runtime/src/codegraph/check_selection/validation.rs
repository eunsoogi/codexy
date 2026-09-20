use std::collections::{BTreeMap, BTreeSet};

use super::model::{
    CheckDefinition, CheckMapping, CheckMappings, GapReason, MappingKind, MappingOwner,
    SelectionGap,
};

pub(super) type MappingKey = (MappingOwner, MappingKind, String);

pub(super) struct MappingValidation {
    pub(super) definitions: BTreeMap<String, CheckDefinition>,
    pub(super) contradictory_checks: BTreeSet<String>,
    pub(super) contradictory_mappings: BTreeSet<MappingKey>,
    pub(super) gaps: BTreeSet<SelectionGap>,
}

pub(super) fn validate(mappings: &CheckMappings) -> MappingValidation {
    let mut definitions = BTreeMap::<String, CheckDefinition>::new();
    let mut contradictory_checks = BTreeSet::new();
    let mut gaps = BTreeSet::new();
    for check in &mappings.checks {
        match definitions.get(&check.id) {
            Some(existing)
                if existing.command != check.command
                    || existing.description != check.description =>
            {
                contradictory_checks.insert(check.id.clone());
                gaps.insert(SelectionGap {
                    path: None,
                    area: None,
                    mapping_pattern: None,
                    reason: GapReason::ContradictoryMapping,
                    detail: format!("check `{}` has conflicting definitions", check.id),
                });
            }
            Some(_) => {}
            None => {
                definitions.insert(check.id.clone(), check.clone());
            }
        }
    }

    let mut seen_mappings = BTreeMap::<MappingKey, Vec<String>>::new();
    let mut contradictory_mappings = BTreeSet::new();
    for mapping in &mappings.mappings {
        let key = mapping_key(mapping);
        let check_ids = canonical_check_ids(&mapping.check_ids);
        match seen_mappings.get(&key) {
            Some(existing_check_ids) if existing_check_ids != &check_ids => {
                contradictory_mappings.insert(key.clone());
                gaps.insert(SelectionGap {
                    path: None,
                    area: None,
                    mapping_pattern: Some(mapping.pattern.clone()),
                    reason: GapReason::ContradictoryMapping,
                    detail: format!(
                        "{} {} mapping `{}` names conflicting checks",
                        owner_label(mapping.owner),
                        mapping_kind_label(mapping.kind),
                        mapping.pattern
                    ),
                });
            }
            Some(_) => {}
            None => {
                seen_mappings.insert(key, check_ids);
            }
        }
    }
    MappingValidation {
        definitions,
        contradictory_checks,
        contradictory_mappings,
        gaps,
    }
}

pub(super) fn mapping_key(mapping: &CheckMapping) -> MappingKey {
    (mapping.owner, mapping.kind, mapping.pattern.clone())
}

fn canonical_check_ids(check_ids: &[String]) -> Vec<String> {
    check_ids
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

const fn owner_label(owner: MappingOwner) -> &'static str {
    match owner {
        MappingOwner::User => "user",
        MappingOwner::Repository => "repository",
    }
}

const fn mapping_kind_label(kind: MappingKind) -> &'static str {
    match kind {
        MappingKind::Path => "path",
        MappingKind::SharedConfiguration => "shared-configuration",
        MappingKind::Fixture => "fixture",
    }
}
