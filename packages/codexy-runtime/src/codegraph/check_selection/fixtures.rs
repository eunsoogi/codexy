use super::model::{CheckDefinition, CheckMapping, CheckMappings, MappingKind, MappingOwner};

pub(super) fn supported_examples() -> CheckMappings {
    CheckMappings::new(
        vec![
            CheckDefinition {
                id: "docs".into(),
                command: "dprint check".into(),
                description: "Check JSON, YAML, and Markdown formatting".into(),
            },
            CheckDefinition {
                id: "codegraph".into(),
                command: "cargo test --manifest-path packages/codexy-runtime/Cargo.toml --locked --lib codegraph".into(),
                description: "Run focused Codegraph library tests".into(),
            },
            CheckDefinition {
                id: "runtime".into(),
                command: "cargo test --manifest-path packages/codexy-runtime/Cargo.toml --locked".into(),
                description: "Run the complete runtime package suite".into(),
            },
        ],
        vec![
            CheckMapping {
                owner: MappingOwner::Repository,
                kind: MappingKind::Path,
                pattern: "docs/".into(),
                check_ids: vec!["docs".into()],
                reason: "documentation formatting is the supported text check".into(),
            },
            CheckMapping {
                owner: MappingOwner::Repository,
                kind: MappingKind::Path,
                pattern: "packages/codexy-runtime/src/codegraph/".into(),
                check_ids: vec!["codegraph".into()],
                reason: "Codegraph modules have focused library coverage".into(),
            },
            CheckMapping {
                owner: MappingOwner::Repository,
                kind: MappingKind::Fixture,
                pattern: "packages/codexy-runtime/tests/fixtures/".into(),
                check_ids: vec!["runtime".into()],
                reason: "shared fixtures can affect the runtime suite".into(),
            },
            CheckMapping {
                owner: MappingOwner::Repository,
                kind: MappingKind::SharedConfiguration,
                pattern: "packages/codexy-runtime/Cargo.toml".into(),
                check_ids: vec!["runtime".into()],
                reason: "shared package configuration changes need broad coverage".into(),
            },
            CheckMapping {
                owner: MappingOwner::Repository,
                kind: MappingKind::SharedConfiguration,
                pattern: "Cargo.lock".into(),
                check_ids: vec!["runtime".into()],
                reason: "resolved dependency changes need broad coverage".into(),
            },
        ],
    )
}
