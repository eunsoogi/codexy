use std::{collections::BTreeSet, path::Path};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const LEGACY_SKILLS: [&str; 6] = [
    "debugging",
    "domain-driven-development",
    "qa",
    "refactoring",
    "spec-driven-development",
    "test-driven-development",
];

const ENGINEERING_REFERENCES: [&str; 6] = [
    "diagnosis.md", "domain-modeling.md", "quality-assurance.md", "refactoring.md",
    "specification.md", "test-driven-development.md",
];

const REQUIRED_SECTIONS: [&str; 6] = [
    "## Diagnosis",
    "## Specification",
    "## Domain modeling",
    "## Test-driven development",
    "## Refactoring",
    "## Quality assurance",
];

#[derive(Clone, Copy)]
struct LegacyCase {
    legacy: &'static str,
    reference: &'static str,
    must_rules: usize,
    outputs: &'static [&'static str],
    contracts: &'static [&'static str],
}

const PARITY_CASES: [LegacyCase; 6] = [
    LegacyCase { legacy: "debugging", reference: "diagnosis.md", must_rules: 18, outputs: &["Symptom:", "Reproduction:", "Expected:", "Actual:", "Hypotheses:", "Experiment:", "Result:", "Fix:", "Regression proof:", "Cleanup:"], contracts: &["plain-language-user-replies.md", "natural-korean-responses.md"] },
    LegacyCase { legacy: "domain-driven-development", reference: "domain-modeling.md", must_rules: 16, outputs: &["Glossary:", "Bounded contexts:", "Owned invariants:", "Boundary adapters:", "Domain errors:", "Proofs:", "Risks:"], contracts: &[] },
    LegacyCase { legacy: "spec-driven-development", reference: "specification.md", must_rules: 13, outputs: &["Spec source:", "Atomic outcome:", "In scope:", "Out of scope:", "Success criteria:", "Proof plan:", "Open questions:"], contracts: &[] },
    LegacyCase { legacy: "test-driven-development", reference: "test-driven-development.md", must_rules: 18, outputs: &["Behavior:", "Root-cause boundary:", "Harness cost:", "Integration target:", "Performance RED:", "RED command:", "RED reason:", "GREEN command:", "Broader verification:", "Refactor notes:", "Not covered:"], contracts: &[] },
    LegacyCase { legacy: "refactoring", reference: "refactoring.md", must_rules: 36, outputs: &["Refactor goal:", "Behavior preserved:", "Touched implementation LOC:", "Governed LOC compliance (all files <=250 LOC):", "Structural remediation rationale:", "Public contracts checked:", "Tests or regression proof:", "Verification:", "Follow-up issues:"], contracts: &[] },
    LegacyCase { legacy: "qa", reference: "quality-assurance.md", must_rules: 16, outputs: &["Claim:", "Channel:", "Invocation:", "Expected observable:", "Evidence:", "Result:", "Cleanup:"], contracts: &["plain-language-user-replies.md", "natural-korean-responses.md"] },
];

#[test]
fn engineering_skill_is_the_only_packaged_route_for_the_six_workflows() -> TestResult {
    let root = codexy_runtime::paths::repository_root();
    let skills = root.join("plugins/codexy/skills");
    let engineering = skills.join("engineering");
    let skill = engineering_documents(&engineering)?;
    let prompt = std::fs::read_to_string(engineering.join("agents/openai.yaml"))?;
    let matrix = std::fs::read_to_string(
        engineering.join("references/legacy-rule-equivalence-matrix.md"),
    )?;

    validate_engineering_skill(&engineering, &skill)?;
    assert!(prompt.contains("$engineering"));
    assert!(prompt.contains("$task-classification"));
    for trigger in ["diagnosis", "specification", "domain modeling", "test-driven development", "refactoring", "quality assurance"] {
        assert!(skill.contains(trigger), "frontmatter or route omits {trigger}");
    }
    for case in PARITY_CASES {
        assert!(matrix.contains(case.legacy), "matrix omits {}", case.legacy);
        assert!(
            !is_skill_bundle(&skills.join(case.legacy)),
            "legacy routing surface remains: {}", case.legacy
        );
    }
    Ok(())
}

#[test]
fn engineering_contract_rejects_each_section_and_invariant_omission() -> TestResult {
    let root = codexy_runtime::paths::repository_root();
    let engineering = root.join("plugins/codexy/skills/engineering");
    let skill = engineering_documents(&engineering)?;
    for required in REQUIRED_SECTIONS {
        assert!(validate_engineering_skill(&engineering, &skill.replacen(required, "", 1)).is_err());
    }
    for case in PARITY_CASES {
        let document = std::fs::read_to_string(engineering.join("references").join(case.reference))?;
        for rule in document.lines().filter(|line| line.contains("MUST")) {
            let missing = document.replacen(rule, "", 1);
            assert!(!valid_case(case, &missing), "omitting {rule:?} must fail");
        }
    }
    Ok(())
}

fn validate_engineering_skill(engineering: &Path, skill: &str) -> Result<(), String> {
    let frontmatter = skill.split("---").nth(1).ok_or("skill frontmatter missing")?;
    let frontmatter: serde_yaml::Value = serde_yaml::from_str(frontmatter)
        .map_err(|error| format!("invalid skill frontmatter: {error}"))?;
    if frontmatter["name"].as_str() != Some("engineering") {
        return Err("engineering skill name missing".to_owned());
    }

    for required in REQUIRED_SECTIONS {
        if !skill.contains(required) {
            return Err(format!("engineering contract omits {required:?}"));
        }
    }
    if !valid_inventory(&PARITY_CASES) { return Err("invalid legacy-rule inventory".to_owned()); }
    for case in PARITY_CASES {
        let document = std::fs::read_to_string(engineering.join("references").join(case.reference))
            .map_err(|error| error.to_string())?;
        if !valid_case(case, &document) { return Err(format!("invalid parity case: {}", case.legacy)); }
    }
    Ok(())
}

fn valid_case(case: LegacyCase, document: &str) -> bool {
    document.lines().filter(|line| line.contains("MUST")).count() == case.must_rules
        && case.outputs.iter().all(|output| document.contains(output))
        && case.contracts.iter().all(|contract| document.contains(contract))
}

fn valid_inventory(cases: &[LegacyCase]) -> bool {
    cases.len() == LEGACY_SKILLS.len()
        && cases.iter().map(|case| case.legacy).collect::<BTreeSet<_>>()
            == LEGACY_SKILLS.into_iter().collect()
        && cases.iter().map(|case| case.reference).collect::<BTreeSet<_>>().len() == cases.len()
        && cases.iter().map(|case| case.reference).collect::<BTreeSet<_>>()
            == ENGINEERING_REFERENCES.into_iter().collect()
}

#[test]
fn legacy_skill_paths_are_not_retained_as_compatibility_aliases() {
    let root = codexy_runtime::paths::repository_root();
    let skills = root.join("plugins/codexy/skills");
    for legacy in LEGACY_SKILLS {
        assert!(!is_skill_bundle(&skills.join(legacy)), "legacy bundle remains: {legacy}");
    }
}

#[test]
fn parity_inventory_rejects_missing_duplicate_unknown_and_stale_entries() {
    assert!(valid_inventory(&PARITY_CASES));
    assert!(!valid_inventory(&PARITY_CASES[..5]));
    let mut duplicate = PARITY_CASES;
    duplicate[5] = duplicate[0];
    assert!(!valid_inventory(&duplicate));
    let mut unknown = PARITY_CASES;
    unknown[0] = LegacyCase { legacy: "unknown", ..unknown[0] };
    assert!(!valid_inventory(&unknown));
    let mut stale = PARITY_CASES;
    stale[0] = LegacyCase { reference: "stale.md", ..stale[0] };
    assert!(!valid_inventory(&stale));
}

fn is_skill_bundle(path: &Path) -> bool {
    path.join("SKILL.md").is_file() || path.join("agents/openai.yaml").is_file()
}

fn engineering_documents(engineering: &Path) -> Result<String, std::io::Error> {
    let mut documents = std::fs::read_to_string(engineering.join("SKILL.md"))?;
    for reference in [
        "diagnosis.md",
        "specification.md",
        "domain-modeling.md",
        "test-driven-development.md",
        "refactoring.md",
        "quality-assurance.md",
    ] {
        documents.push('\n');
        documents.push_str(&std::fs::read_to_string(engineering.join("references").join(reference))?);
    }
    Ok(documents)
}
