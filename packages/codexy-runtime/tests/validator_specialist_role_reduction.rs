use std::collections::BTreeSet;

use crate::support::TestResult;

const RETAINED: [&str; 7] = [
    "codexy-architect",
    "codexy-auditor",
    "codexy-cartographer",
    "codexy-sentinel",
    "codexy-shipwright",
    "codexy-warden",
    "codexy-weaver",
];
const RETIRED: [&str; 5] = [
    "codexy-forge",
    "codexy-pathfinder",
    "codexy-scribe",
    "codexy-sculptor",
    "codexy-tracer",
];
const ROLE_EQUIVALENCE: [(&str, &str, &str); 12] = [
    ("codexy-architect", "Retain", "Architecture and durable schema boundaries."),
    ("codexy-auditor", "Retain", "Acceptance evidence and observable QA."),
    ("codexy-cartographer", "Retain", "Read-only repository and ownership mapping."),
    ("codexy-forge", "Remove", "The generic owning child performs scoped implementation."),
    ("codexy-pathfinder", "Remove", "Orchestration owns classification, planning, and approach selection."),
    ("codexy-scribe", "Remove", "The owning child drafts its own documentation and handoff."),
    ("codexy-sculptor", "Remove", "The engineering workflow owns behavior-preserving refactoring."),
    ("codexy-sentinel", "Retain", "Independent strict review; fixed at `gpt-5.6-sol` / `xhigh`."),
    ("codexy-shipwright", "Retain", "Release, package, and rollback readiness."),
    ("codexy-tracer", "Remove", "The engineering workflow owns diagnosis and regression investigation."),
    ("codexy-warden", "Retain", "Security, permission, shell, and state-mutation boundaries."),
    ("codexy-weaver", "Retain", "GitHub/integration contract; its future physical move belongs to the GitHub-plugin work."),
];

#[test]
fn role_equivalence_records_the_exact_reduction() -> TestResult {
    let root = codexy_runtime::paths::repository_root();
    let catalog = std::fs::read_to_string(root.join("plugins/codexy/agents/catalog.toml"))?;
    let configured = toml::from_str::<toml::Value>(&catalog)?["agent_files"]
        .as_array()
        .ok_or("catalog agent_files")?
        .iter()
        .map(|value| value.as_str().map(str::to_owned).ok_or("catalog filename"))
        .collect::<Result<BTreeSet<_>, _>>()?;
    let expected = RETAINED
        .iter()
        .map(|name| format!("{name}.toml"))
        .collect::<BTreeSet<_>>();
    assert_eq!(configured, expected);

    let mapping = std::fs::read_to_string(root.join("docs/specialist-role-equivalence.md"))?;
    let rows = role_rows(&mapping)?;
    assert_eq!(rows.len(), ROLE_EQUIVALENCE.len());
    for (name, disposition, owner) in ROLE_EQUIVALENCE {
        assert_eq!(
            rows.get(name),
            Some(&(disposition.to_owned(), owner.to_owned())),
            "role-equivalence row for {name}"
        );
    }
    assert!(mapping.contains("`codexy-inspector` is reserved for #562"));
    assert!(!catalog.contains("codexy-inspector"));
    Ok(())
}

fn role_rows(
    mapping: &str,
) -> Result<std::collections::BTreeMap<String, (String, String)>, Box<dyn std::error::Error>> {
    let mut rows = std::collections::BTreeMap::new();
    for line in mapping.lines().filter(|line| line.starts_with("| `codexy-")) {
        let cells = line
            .trim_matches('|')
            .split('|')
            .map(str::trim)
            .collect::<Vec<_>>();
        if cells.len() != 3 {
            return Err(format!("invalid role-equivalence row: {line}").into());
        }
        let name = cells[0].trim_matches('`').to_owned();
        if rows
            .insert(name.clone(), (cells[1].to_owned(), cells[2].to_owned()))
            .is_some()
        {
            return Err(format!("duplicate role-equivalence row: {name}").into());
        }
    }
    Ok(rows)
}

#[test]
fn active_routing_does_not_recreate_retired_roles_as_aliases() -> TestResult {
    let root = codexy_runtime::paths::repository_root();
    let routing = std::fs::read_to_string(root.join("plugins/codexy/skills/orchestration/SKILL.md"))?;
    let child_control = std::fs::read_to_string(
        root.join("plugins/codexy/skills/orchestration/references/classification-and-control.md"),
    )?;
    for name in RETIRED {
        assert!(!routing.contains(name), "retired role remains in active routing: {name}");
        assert!(
            !child_control.contains(name),
            "retired role remains in child routing: {name}"
        );
    }
    assert!(!routing.contains("codexy-inspector"));
    assert!(!child_control.contains("codexy-inspector"));
    Ok(())
}
