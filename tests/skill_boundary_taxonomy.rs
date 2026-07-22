use std::collections::BTreeSet;
use std::path::Path;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn every_packaged_skill_has_one_keep_decision_and_stable_identity() -> TestResult {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let guide = std::fs::read_to_string(root.join("docs/architecture.md"))?;
    let rows = table_rows(section(&guide, "Packaged skills")?);
    let skills_root = root.join("plugins/codexy/skills");
    let mut packaged = BTreeSet::new();

    for entry in std::fs::read_dir(&skills_root)? {
        let folder = entry?.file_name().to_string_lossy().into_owned();
        let skill_path = skills_root.join(&folder).join("SKILL.md");
        if !skill_path.is_file() {
            continue;
        }
        let text = std::fs::read_to_string(&skill_path)?;
        let frontmatter = text.split("---").nth(1).ok_or("skill frontmatter missing")?;
        let value: serde_yaml::Value = serde_yaml::from_str(frontmatter)?;
        let name = value["name"].as_str().ok_or("skill name missing")?;
        let description = value["description"]
            .as_str()
            .ok_or("skill description missing")?;
        assert_eq!(folder, name, "folder and frontmatter name differ");
        assert!(name.len() < 64 && is_lower_hyphenated(name));
        assert!(!description.trim().is_empty());
        packaged.insert(name.to_owned());
    }

    assert_eq!(rows.len(), packaged.len());
    let documented = rows
        .iter()
        .map(|row| {
            assert_eq!(row.len(), 4, "skill rows need name, decision, trigger, responsibility");
            assert_eq!(row[1], "Keep", "unsupported taxonomy churn for {}", row[0]);
            assert!(!row[2].is_empty() && !row[3].is_empty());
            row[0].clone()
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(documented, packaged);
    Ok(())
}

#[test]
fn overlap_boundaries_and_non_markdown_authority_are_explicit() -> TestResult {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let guide = std::fs::read_to_string(root.join("docs/architecture.md"))?;
    let boundaries = data_rows(subsection(
        &guide,
        "### Overlap boundaries",
        "## Skill path-consumer map",
    )?)
    .into_iter()
    .map(|row| row[0].clone())
    .collect::<BTreeSet<_>>();
    assert_eq!(
        boundaries,
        [
            "Change method and diagnosis",
            "Packaging and release",
            "Planning and domain ownership",
            "Routing, execution, and context",
            "Verification and completion",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect()
    );

    let consumers = data_rows(section(&guide, "Skill path-consumer map")?)
        .into_iter()
        .map(|row| row[0].clone())
        .collect::<BTreeSet<_>>();
    assert_eq!(consumers.len(), 8);
    for consumer in [
        "Host discovery",
        "Skill prompt metadata",
        "Plugin entry prompt",
        "Recursive instruction validation",
        "Path-specific policy validation",
        "Inventory and taxonomy tests",
        "Structured contracts",
        "Skill resources",
    ] {
        assert!(consumers.get(consumer).is_some());
    }

    let classification = std::fs::read_to_string(
        root.join("plugins/codexy/skills/task-classification/SKILL.md"),
    )?;
    assert!(section(&classification, "Authority Boundary")?
        .lines()
        .any(|line| !line.trim().is_empty()));
    Ok(())
}

#[test]
fn gfm_owner_decision_remains_non_authoritative_without_lane_metadata() -> TestResult {
    let partial_table = r#"| Field | Value |
| --- | --- |
| Lane type | implementation |
| Secondary surfaces | validators |
| Owner decision | current-thread-owned child implementation lane |
"#;
    let complete_table = format!(
        "{partial_table}{}",
        r#"| Atomic scope | issue-sized |
| Required skills | task-classification |
| Required tools/evidence | goal, plan |
| First allowed action | implement after classification |
| Stop/blocker | None |
"#
    );
    let table_only =
        run_ownership_validator(&format!("{partial_table}Plan tool call: update_plan\n"))?;
    assert!(
        table_only.status.success(),
        "a partial GFM owner row must not establish child control authority: {}",
        String::from_utf8_lossy(&table_only.stderr)
    );

    let authoritative_child = run_ownership_validator(&format!(
        "Lane ownership: child-owned\n{partial_table}Plan tool call: update_plan\n"
    ))?;
    assert!(!authoritative_child.status.success());

    let missing_metadata = run_ownership_validator(&format!(
        "{complete_table}Plan tool call: update_plan\n"
    ))?;
    assert!(
        !missing_metadata.status.success(),
        "a complete display table must not establish authority without metadata"
    );

    let malformed_metadata = run_ownership_validator(&format!(
        "Ownership metadata source: parent-supplied\nLane ownership: unknown\n{complete_table}Plan tool call: update_plan\n"
    ))?;
    assert!(
        !malformed_metadata.status.success(),
        "malformed authoritative ownership metadata must be rejected"
    );

    let classified_child = run_ownership_validator(&format!(
        "Ownership metadata source: parent-supplied\nLane ownership: child-owned\nTask classification:\n{complete_table}Plan tool call: update_plan\n"
    ))?;
    assert!(classified_child.status.success());
    Ok(())
}

fn run_ownership_validator(evidence: &str) -> Result<std::process::Output, Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let path = temp.path().join("handoff.md");
    std::fs::write(&path, evidence)?;
    Ok(crate::support::validator_child_lane_ownership_file(&path)?)
}

fn section<'a>(guide: &'a str, heading: &str) -> Result<&'a str, String> {
    let marker = format!("## {heading}");
    let start = guide.find(&marker).ok_or_else(|| format!("missing heading: {marker}"))?;
    let remainder = &guide[start + marker.len()..];
    Ok(remainder.split("\n## ").next().unwrap_or(remainder))
}

fn table_rows(section: &str) -> Vec<Vec<String>> {
    section
        .lines()
        .filter(|line| line.starts_with("| `"))
        .map(|line| {
            line.trim_matches('|')
                .split('|')
                .map(|cell| cell.trim().trim_matches('`').to_owned())
                .collect()
        })
        .collect()
}

fn subsection<'a>(text: &'a str, start: &str, end: &str) -> Result<&'a str, String> {
    let (_, remainder) = text
        .split_once(start)
        .ok_or_else(|| format!("missing subsection: {start}"))?;
    remainder
        .split_once(end)
        .map(|(body, _)| body)
        .ok_or_else(|| format!("missing subsection end: {end}"))
}

fn data_rows(section: &str) -> Vec<Vec<String>> {
    section
        .lines()
        .filter(|line| line.starts_with('|'))
        .skip(2)
        .map(|line| {
            line.trim_matches('|')
                .split('|')
                .map(|cell| cell.trim().trim_matches('`').to_owned())
                .collect()
        })
        .collect()
}

fn is_lower_hyphenated(name: &str) -> bool {
    !name.starts_with('-')
        && !name.ends_with('-')
        && !name.contains("--")
        && name
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}
