use crate::support::TestResult;

// Keep both introductions tied to the same requested-role contract, without
// treating documentation or source configuration as effective host telemetry.
#[test]
fn bilingual_role_tables_match_current_model_routes() -> TestResult {
    let root = codexy_runtime::paths::repository_root();
    let advisor = super::parse_toml(&root.join("plugins/codexy/agents/codexy-advisor.toml"))?;
    let advisor_model = super::text_field(&advisor, "model")?;
    let advisor_effort = super::text_field(&advisor, "model_reasoning_effort")?;
    for (file, heading) in [
        ("README.md", "Model roles and reasoning effort"),
        ("README.ko.md", "모델 역할과 추론 수준"),
    ] {
        let text = std::fs::read_to_string(root.join(file))?;
        let rows: Vec<Vec<&str>> = super::section(&text, heading)?
            .lines()
            .filter(|line| line.starts_with('|'))
            .skip(2)
            .map(|line| {
                line.trim_matches('|')
                    .split('|')
                    .map(|cell| cell.trim().trim_matches('`'))
                    .collect()
            })
            .collect();
        let expected = [
            ("gpt-6.2-sol", "low"),
            (advisor_model.as_str(), advisor_effort.as_str()),
            ("gpt-6.1-terra", "medium"),
            ("gpt-6.2-sol", "medium"),
        ];
        assert_eq!(rows.len(), expected.len(), "{file}");
        for (row, (model, effort)) in rows.iter().zip(expected) {
            assert_eq!(row.len(), 4, "{file}");
            assert_eq!((row[1], row[2]), (model, effort), "{file}");
        }
        super::assert_local_links(root, &root.join(file), &text)?;
    }
    let guide_path = root.join("docs/model-migration.md");
    let guide = std::fs::read_to_string(&guide_path)?;
    super::assert_local_links(root, &guide_path, &guide)?;
    Ok(())
}
