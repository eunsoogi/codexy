use std::path::Path;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const LEGACY_SKILLS: [&str; 6] = [
    "debugging",
    "domain-driven-development",
    "qa",
    "refactoring",
    "spec-driven-development",
    "test-driven-development",
];

const REQUIRED_SECTIONS: [&str; 6] = [
    "## Diagnosis",
    "## Specification",
    "## Domain modeling",
    "## Test-driven development",
    "## Refactoring",
    "## Quality assurance",
];

const REQUIRED_INVARIANTS: [&str; 18] = [
    "MUST reproduce the symptom",
    "MUST test one hypothesis at a time",
    "MUST remove temporary instrumentation",
    "MUST define proofs before implementation",
    "MUST NOT widen scope",
    "MUST map each changed file back to the requirement",
    "MUST build a glossary",
    "MUST identify bounded contexts",
    "MUST capture invariants",
    "MUST run the proof before implementation and capture RED",
    "MUST confirm RED fails because the behavior is missing or wrong",
    "MUST run the same proof and capture GREEN",
    "MUST keep code files at or below 250 lines of code by default",
    "MUST NOT change behavior silently",
    "MUST move code while preserving public contracts",
    "MUST list claims that need proof",
    "MUST drive the real surface",
    "MUST NOT call a scenario PASS without direct evidence",
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

    validate_engineering_skill(&skill)?;
    assert!(prompt.contains("$engineering"));
    for legacy in LEGACY_SKILLS {
        assert!(matrix.contains(legacy), "matrix omits {legacy}");
        assert!(
            !is_skill_bundle(&skills.join(legacy)),
            "legacy routing surface remains: {legacy}"
        );
    }
    Ok(())
}

#[test]
fn engineering_contract_rejects_each_section_and_invariant_omission() -> TestResult {
    let root = codexy_runtime::paths::repository_root();
    let skill = engineering_documents(&root.join("plugins/codexy/skills/engineering"))?;

    for required in REQUIRED_SECTIONS.into_iter().chain(REQUIRED_INVARIANTS) {
        let without_required = skill.replacen(required, "", 1);
        assert!(
            validate_engineering_skill(&without_required).is_err(),
            "omitting {required:?} must invalidate the engineering contract"
        );
    }
    Ok(())
}

fn validate_engineering_skill(skill: &str) -> Result<(), String> {
    let frontmatter = skill.split("---").nth(1).ok_or("skill frontmatter missing")?;
    let frontmatter: serde_yaml::Value = serde_yaml::from_str(frontmatter)
        .map_err(|error| format!("invalid skill frontmatter: {error}"))?;
    if frontmatter["name"].as_str() != Some("engineering") {
        return Err("engineering skill name missing".to_owned());
    }

    for required in REQUIRED_SECTIONS.into_iter().chain(REQUIRED_INVARIANTS) {
        if !skill.contains(required) {
            return Err(format!("engineering contract omits {required:?}"));
        }
    }
    Ok(())
}

#[test]
fn legacy_skill_paths_are_not_retained_as_compatibility_aliases() {
    let root = codexy_runtime::paths::repository_root();
    let skills = root.join("plugins/codexy/skills");
    for legacy in LEGACY_SKILLS {
        assert!(!is_skill_bundle(&skills.join(legacy)), "legacy bundle remains: {legacy}");
    }
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
