use serde_yaml::Value;

use super::wiki_minimal_contract_markdown::Document;

pub(crate) fn validate_core_skill(source: &str, removed: &[&str]) -> Result<(), String> {
    let document = Document::parse(source)?;
    let workflow = document.section("## Core workflow")?;
    required_count(
        document.inline_code_count(Some(&workflow), "init → ingest → compile → query → refresh"),
        "core command inventory",
    )?;
    required_count(
        document.link_count("Migration", "references/migration.md"),
        "migration link",
    )?;
    for command in removed {
        if document.inline_code_count(None, command) != 0 {
            return Err(format!("removed command remains active: {command}"));
        }
    }
    Ok(())
}

pub(crate) fn markdown_link_count(
    source: &str,
    label: &str,
    target: &str,
) -> Result<usize, String> {
    Ok(Document::parse(source)?.link_count(label, target))
}

pub(crate) fn frontmatter_string(source: &str, key: &str) -> Result<String, String> {
    let source = source.strip_prefix('\u{feff}').unwrap_or(source);
    let (opening, remainder) = source.split_once('\n').ok_or("frontmatter opening")?;
    if opening.trim_end_matches('\r') != "---" {
        return Err("frontmatter opening".into());
    }
    let (_, frontmatter) = remainder.split_once("\n---").ok_or("frontmatter closing")?;
    let yaml = &remainder[..remainder.len() - frontmatter.len() - 4];
    let Value::Mapping(mapping) =
        serde_yaml::from_str::<Value>(yaml).map_err(|error| error.to_string())?
    else {
        return Err("frontmatter mapping".into());
    };
    mapping
        .get(Value::String(key.into()))
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| format!("frontmatter string: {key}"))
}

pub(crate) fn validate_migration_rules(source: &str) -> Result<(), String> {
    let shape = NormalizedRules::new(source);
    let rules: &[&[&str]] = &[
        &["must preserve existing", "raw", "wiki", "index", "log"],
        &[
            "must not delete",
            "overwrite",
            "rename",
            "existing topic data",
        ],
        &[
            "must preserve every complete",
            "relative",
            "sources scalar",
            "exactly",
        ],
        &[
            "must stop",
            "must report the provenance gap",
            "must leave the entire topic tree unchanged",
        ],
        &[
            "must validate every referenced provenance",
            "freshness input",
            "before any log",
            "derived write",
        ],
    ];
    if rules.iter().any(|rule| !shape.has_required_concepts(rule))
        || !shape.has_ordered_concepts(&[
            "must validate every referenced provenance and freshness input before any log or derived write",
            "must append one migration entry",
        ])
    {
        return Err("migration rule identity".into());
    }
    Ok(())
}

fn required_count(count: usize, identity: &str) -> Result<(), String> {
    (count == 1)
        .then_some(())
        .ok_or_else(|| format!("missing or duplicate {identity}"))
}

struct NormalizedRules(String);

impl NormalizedRules {
    fn new(source: &str) -> Self {
        Self(
            source
                .to_ascii_lowercase()
                .split(|character: char| !character.is_ascii_alphanumeric())
                .filter(|token| !token.is_empty())
                .collect::<Vec<_>>()
                .join(" "),
        )
    }

    fn has_required_concepts(&self, required: &[&str]) -> bool {
        required
            .iter()
            .all(|concept| phrase(&self.0, &normalize(concept)))
    }

    fn has_ordered_concepts(&self, required: &[&str]) -> bool {
        let mut offset = 0;
        for concept in required {
            let concept = normalize(concept);
            let Some(index) = self.0[offset..]
                .match_indices(&concept)
                .find_map(|(index, _)| {
                    phrase_at(&self.0, offset + index, &concept).then_some(index)
                })
            else {
                return false;
            };
            offset += index + concept.len();
        }
        true
    }
}

fn normalize(source: &str) -> String {
    source
        .to_ascii_lowercase()
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|token| !token.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn phrase(text: &str, concept: &str) -> bool {
    text.match_indices(concept)
        .any(|(index, _)| phrase_at(text, index, concept))
}

fn phrase_at(text: &str, index: usize, concept: &str) -> bool {
    let before = text[..index].chars().next_back();
    let after = text[index + concept.len()..].chars().next();
    before.is_none_or(char::is_whitespace) && after.is_none_or(char::is_whitespace)
}
