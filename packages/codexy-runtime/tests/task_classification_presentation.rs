
type TestResult = Result<(), Box<dyn std::error::Error>>;

use std::collections::BTreeSet;
use std::path::Path;

const FIELDS: [&str; 8] = [
    "Lane type",
    "Secondary surfaces",
    "Owner decision",
    "Atomic scope",
    "Required skills",
    "Required tools/evidence",
    "First allowed action",
    "Stop/blocker",
];

#[test]
fn orchestration_owns_formal_classification_tables_only_when_required() -> TestResult {
    let root = codexy_runtime::paths::repository_root();
    let skill = std::fs::read_to_string(
        root.join("plugins/codexy/skills/orchestration/references/task-classification.md"),
    )?;
    let table = formal_output_table(&skill)?;

    assert_eq!(field_names(table)?, FIELDS);
    assert!(field_names(&table.replacen("| Stop/blocker |", "", 1)).is_err());
    assert!(field_names(&table.replace(
        "| Stop/blocker |",
        "| First allowed action |"
    ))
    .is_err());
    assert!(field_names(&table.replace(
        "| Lane type |",
        "| Secondary surfaces |"
    ))
    .is_err());
    assert!(field_names(&table.replacen(
        "| Lane type |",
        "| __swap__ |",
        1
    ).replacen(
        "| Secondary surfaces |",
        "| Lane type |",
        1
    ).replacen("| __swap__ |", "| Secondary surfaces |", 1))
    .is_err());

    let prompt = std::fs::read_to_string(
        root.join("plugins/codexy/skills/orchestration/agents/openai.yaml"),
    )?;
    let prompt: serde_yaml::Value = serde_yaml::from_str(&prompt)?;
    let default_prompt = prompt["interface"]["default_prompt"]
        .as_str()
        .ok_or("missing orchestration default prompt")?;
    assert_eq!(
        default_prompt,
        "You MUST use $orchestration first to select the light, standard, or strict workflow profile. Light is the default for read-only, documentation, tiny fixes, and ordinary single-owner mutations; standard covers non-trivial single-owner work. Light and standard MUST NOT require a visible eight-row table, goal/plan receipts, or skip rationales. Strict work, durable delegation, multi-lane ownership, and explicit audit evidence MUST render the ordered formal classification table before setup, delegation, implementation, PR, review-response, or merge work begins."
    );
    Ok(())
}

#[test]
fn relocated_orchestration_references_resolve_from_their_new_locations() -> TestResult {
    let root = codexy_runtime::paths::repository_root();
    let references = root.join("plugins/codexy/skills/orchestration/references");

    for entry in std::fs::read_dir(&references)? {
        let path = entry?.path();
        if path.extension().is_some_and(|extension| extension == "md") {
            assert_local_links(&path, &std::fs::read_to_string(&path)?)?;
        }
    }

    let relocated = references.join("task-classification.md");
    let original = std::fs::read_to_string(&relocated)?;
    assert!(assert_local_links(
        &relocated,
        &original.replacen(
            "workflow-profiles.json",
            "orchestration/references/workflow-profiles.json",
            1,
        ),
    )
    .is_err());
    let token_efficient = references.join("token-efficient.md");
    assert!(assert_local_links(
        &token_efficient,
        &std::fs::read_to_string(&token_efficient)?.replacen(
            "../templates/delta-poll.md",
            "templates/delta-poll.md",
            1,
        ),
    )
    .is_err());
    Ok(())
}

#[test]
fn consolidated_skill_routes_are_unique_across_the_fixture_inventory() -> TestResult {
    let root = codexy_runtime::paths::repository_root();
    let fixtures = root.join("packages/codexy-runtime/tests");

    for entry in std::fs::read_dir(fixtures)? {
        let path = entry?.path();
        if path.extension().is_some_and(|extension| extension == "rs") {
            assert_unique_skill_routes(&path, &std::fs::read_to_string(&path)?)?;
        }
    }

    assert!(unique_skill_routes("orchestration, orchestration, git-workflow").is_err());
    assert_eq!(
        unique_skill_routes("orchestration, git-workflow")?,
        ["git-workflow", "orchestration"].into_iter().collect()
    );
    Ok(())
}

fn formal_output_table(skill: &str) -> Result<&str, String> {
    let section = skill
        .split_once("## Required Output")
        .map(|(_, section)| section)
        .ok_or("missing Required Output section")?;
    section
        .split_once("## Formal Classification Output")
        .map(|(table, _)| table)
        .ok_or_else(|| "missing Formal Classification Output section".to_owned())
}

fn field_names(table: &str) -> Result<Vec<&str>, String> {
    let rows: Vec<_> = table
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with('|'))
        .collect();
    if rows.first() != Some(&"| Field | Value |")
        || rows.get(1) != Some(&"| --- | --- |")
        || rows.len() != FIELDS.len() + 2
    {
        return Err("classification table has the wrong header or row count".to_owned());
    }

    let names: Vec<_> = rows[2..]
        .iter()
        .map(|row| {
            let cells: Vec<_> = row
                .trim_matches('|')
                .split('|')
                .map(str::trim)
                .collect();
            if cells.len() == 2 && !cells[1].is_empty() {
                Ok(cells[0])
            } else {
                Err("classification row is not a populated two-column row".to_owned())
            }
        })
        .collect::<Result<_, _>>()?;
    if names != FIELDS {
        return Err("classification fields are missing, duplicated, or out of order".to_owned());
    }
    Ok(names)
}

fn assert_local_links(path: &Path, text: &str) -> Result<(), String> {
    let base = path.parent().ok_or("document has no parent")?;
    for remainder in text.split("](").skip(1) {
        let target = remainder.split(')').next().ok_or("unterminated link")?;
        if target.starts_with("http://") || target.starts_with("https://") || target.starts_with('#') {
            continue;
        }
        let local = target.split('#').next().unwrap_or(target);
        if !base.join(local).exists() {
            return Err(format!("broken local link from {} to {target}", path.display()));
        }
    }
    Ok(())
}

fn assert_unique_skill_routes(path: &Path, text: &str) -> Result<(), String> {
    let fixture_text = text.replace(r"\n", "\n");
    for (line_number, line) in fixture_text.lines().enumerate() {
        if let Some((_, skills)) = line.split_once("Required skills: ") {
            unique_skill_routes(skills).map_err(|error| {
                format!("{}:{} has an invalid required-skills route: {error}", path.display(), line_number + 1)
            })?;
        }
    }
    Ok(())
}

fn unique_skill_routes(skills: &str) -> Result<BTreeSet<&str>, String> {
    let routes = skills.split(',').map(str::trim).collect::<Vec<_>>();
    let unique = routes.iter().copied().collect::<BTreeSet<_>>();
    if routes.iter().any(|route| route.is_empty()) || unique.len() != routes.len() {
        return Err(format!("duplicate or empty route in {skills:?}"));
    }
    Ok(unique)
}
