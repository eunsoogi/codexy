use serde_json::Value;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const LP_CORPUS: [(&str, &str); 20] = [
    ("LP-P01", "catalog-39"),
    ("LP-P02", "catalog-six-fields"),
    ("LP-P03", "catalog-sorted-ids"),
    ("LP-P04", "json-39"),
    ("LP-P05", "json-three-fields-sorted"),
    ("LP-P06", "semantic-projection"),
    ("LP-P07", "smoke-9"),
    ("LP-P08", "lazy-30"),
    ("LP-P09", "required-extensions"),
    ("LP-P10", "deterministic-projection"),
    ("LP-N01", "DUPLICATE_ID"),
    ("LP-N02", "ID_SET_MISMATCH"),
    ("LP-N03", "PROJECTION_DRIFT"),
    ("LP-N04", "COMMAND_DRIFT"),
    ("LP-N05", "PRIORITY_DRIFT"),
    ("LP-N06", "EXTENSION_DRIFT"),
    ("LP-N07", "SMOKE_EXTENSION_MISSING"),
    ("LP-N08", "UNSUPPORTED_JSON_KEY"),
    ("LP-N09", "EMPTY_COMMAND"),
    ("LP-N10", "UNKNOWN_JSON_ID"),
];

#[test]
fn lsp_projection_corpus_is_literal_and_closed() -> TestResult {
    let root = codexy_runtime::paths::repository_root();
    let catalog_text =
        std::fs::read_to_string(root.join("plugins/codexy-devtools/lsp/server-catalog.toml"))?;
    let config_text =
        std::fs::read_to_string(root.join("plugins/codexy-devtools/.codex/lsp-client.json"))?;
    let validator =
        std::fs::read_to_string(root.join("packages/codexy-runtime/src/validation/lsp.rs"))?;
    let catalog: toml::Value = toml::from_str(&catalog_text)?;
    let servers = catalog["servers"].as_array().ok_or("servers")?;
    let config: Value = serde_json::from_str(&config_text)?;
    let lsp = config["lsp"].as_object().ok_or("lsp")?;
    assert_eq!(servers.len(), 39, "LP-P01");
    assert!(
        servers
            .iter()
            .all(|server| server.as_table().is_some_and(|table| table.len() == 6)),
        "LP-P02"
    );
    let ids: Vec<_> = servers
        .iter()
        .filter_map(|server| server["id"].as_str())
        .collect();
    let mut sorted_ids = ids.clone();
    sorted_ids.sort_unstable();
    assert_eq!(ids, sorted_ids, "LP-P03");
    assert_eq!(lsp.len(), 39, "LP-P04");
    let json_ids: Vec<_> = lsp.keys().map(String::as_str).collect();
    let mut sorted_json_ids = json_ids.clone();
    sorted_json_ids.sort_unstable();
    assert!(
        json_ids == sorted_json_ids
            && lsp
                .values()
                .all(|entry| entry.as_object().is_some_and(|object| object.len() == 3)),
        "LP-P05"
    );
    assert_eq!(ids, json_ids, "LP-P06");
    let smoke = [
        "rust-analyzer",
        "basedpyright",
        "yaml-ls",
        "json-language-server",
        "taplo",
        "marksman",
        "html-language-server",
        "css-language-server",
        "graphql-language-service",
    ];
    assert_eq!(
        servers
            .iter()
            .filter(|server| smoke.contains(&server["id"].as_str().unwrap_or_default()))
            .count(),
        smoke.len(),
        "LP-P07"
    );
    assert_eq!(servers.len() - smoke.len(), 30, "LP-P08");
    assert!(
        [
            ".py", ".pyi", ".yaml", ".yml", ".json", ".toml", ".md", ".html", ".css", ".scss",
            ".less", ".graphql", ".gql"
        ]
        .iter()
        .all(|extension| config_text.contains(extension)),
        "LP-P09"
    );
    let round_trip: Value = serde_json::from_str(&serde_json::to_string(&config)?)?;
    assert_eq!(config, round_trip, "LP-P10");
    for &(case, diagnostic) in &LP_CORPUS[10..] {
        assert!(
            validator.contains(diagnostic),
            "{case} must retain {diagnostic}"
        );
    }
    Ok(())
}
