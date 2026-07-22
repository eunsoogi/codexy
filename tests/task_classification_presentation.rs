use std::path::Path;

type TestResult = Result<(), Box<dyn std::error::Error>>;

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
fn task_classification_uses_one_ordered_table() -> TestResult {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let skill =
        std::fs::read_to_string(root.join("plugins/codexy/skills/task-classification/SKILL.md"))?;
    let table = required_output_table(&skill)?;

    assert_eq!(field_names(table)?, FIELDS);
    assert!(field_names(&table.replacen("| Stop/blocker |", "", 1)).is_err());
    assert!(field_names(&table.replace("| Stop/blocker |", "| First allowed action |")).is_err());
    assert!(field_names(&table.replace("| Lane type |", "| Secondary surfaces |")).is_err());
    assert!(
        field_names(
            &table
                .replacen("| Lane type |", "| __swap__ |", 1)
                .replacen("| Secondary surfaces |", "| Lane type |", 1)
                .replacen("| __swap__ |", "| Secondary surfaces |", 1)
        )
        .is_err()
    );

    let prompt = std::fs::read_to_string(
        root.join("plugins/codexy/skills/task-classification/agents/openai.yaml"),
    )?;
    let prompt: serde_yaml::Value = serde_yaml::from_str(&prompt)?;
    let default_prompt = prompt["interface"]["default_prompt"]
        .as_str()
        .ok_or("missing task-classification default prompt")?;
    assert_eq!(
        default_prompt,
        "You MUST use $task-classification first to render one ordered two-column GFM table with the canonical header row | Field | Value | and exactly these eight ordered rows: Lane type, Secondary surfaces, Owner decision, Atomic scope, Required skills, Required tools/evidence, First allowed action, Stop/blocker; you MUST complete it before Codexy setup, delegation, implementation, PR, review-response, or merge work begins."
    );
    Ok(())
}

fn required_output_table(skill: &str) -> Result<&str, String> {
    let section = skill
        .split_once("## Required Output")
        .map(|(_, section)| section)
        .ok_or("missing Required Output section")?;
    section
        .split_once("## Classification Output")
        .map(|(table, _)| table)
        .ok_or_else(|| "missing Classification Output section".to_owned())
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
            let cells: Vec<_> = row.trim_matches('|').split('|').map(str::trim).collect();
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
