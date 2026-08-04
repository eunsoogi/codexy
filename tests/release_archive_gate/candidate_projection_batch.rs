use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use tempfile::TempDir;

use super::super::{
    candidate::{make_candidate_proven_windows_package, run_source_projection},
    complete_plugin_fixture,
};
use crate::support::FixtureCommand;

const RESET_PATHS: [&str; 5] = [
    ".codex-plugin/plugin.json",
    "mcp/codexy-mcp-codegraph",
    "mcp/codexy-mcp-lsp",
    "runtime-candidate.json",
    "runtime-release.json",
];

const CASES: [(&str, bool); 23] = [
    (
        "bundled_platforms=\"darwin-arm64 linux-x86_64 windows-x86_64\"",
        false,
    ),
    (
        "export bundled_platforms=\"darwin-arm64 linux-x86_64 windows-x86_64\"",
        false,
    ),
    (
        ":; bundled_platforms=\"darwin-arm64 linux-x86_64 windows-x86_64\"",
        false,
    ),
    ("eval 'bundled_platforms=darwin-arm64'", false),
    ("\"eval\" 'bundled_platforms=darwin-arm64'", false),
    ("'eval' 'bundled_platforms=darwin-arm64'", false),
    ("true && eval 'bundled_''platforms=darwin-arm64'", false),
    ("eval 'bundled_''platforms=darwin-arm64' && true", false),
    ("\"ev\"\"al\" 'bundled_''platforms=darwin-arm64'", false),
    (
        "command \"ev\"\"al\" 'bundled_''platforms=darwin-arm64'",
        false,
    ),
    (
        "runner=eval\n$runner 'bundled_''platforms=darwin-arm64'",
        false,
    ),
    (
        "true && runner=eval\n$runner 'bundled_''platforms=darwin-arm64'",
        false,
    ),
    (
        "runner=eval\n\"$runner\" 'bundled_''platforms=darwin-arm64'",
        false,
    ),
    (
        "runner=eval\n${runner} 'bundled_''platforms=darwin-arm64'",
        false,
    ),
    (
        "runner=eval\ncommand \"$runner\" 'bundled_''platforms=darwin-arm64'",
        false,
    ),
    (
        "runner=val\ne$runner 'bundled_''platforms=darwin-arm64'",
        false,
    ),
    (
        "runner=val\n\"e${runner}\" 'bundled_''platforms=darwin-arm64'",
        false,
    ),
    ("`printf eval` 'bundled_''platforms=darwin-arm64'", false),
    (
        "runner=eval\nbuiltin \"$runner\" 'bundled_''platforms=darwin-arm64'",
        false,
    ),
    (
        "# bundled_platforms=\"darwin-arm64 linux-x86_64 windows-x86_64\"",
        true,
    ),
    ("printf '%s\\n' 'bundled_platforms=darwin-arm64'", true),
    ("printf '%s\\n' 'bundled_''platforms=darwin-arm64'", true),
    ("runner=eval\nprintf '%s\\n' \"$runner\"", true),
];

pub(super) fn assert_projection_matrix() {
    let fixture = BatchFixture::new();
    let results = fixture.run(cases_document(&CASES));
    assert_projection_results(&results);
    fixture.assert_reset();
    assert_direct_cli_parity(&fixture, &results, 0, 19);
    assert_invalid_contracts();
}

fn assert_invalid_contracts() {
    let fixture = BatchFixture::new();
    for document in [
        json!({"cases": []}),
        json!({
            "expectedCaseCount": 2,
            "resetPaths": RESET_PATHS,
            "cases": [
                {"id": "first", "append": "one"},
                {"id": "first", "append": "two"},
            ],
        }),
        batch_document(&CASES, json!([RESET_PATHS[0]]), CASES.len()),
        batch_document(&CASES, json!(["../runtime-release.json"]), CASES.len()),
        batch_document(&CASES, json!(RESET_PATHS), CASES.len() - 1),
    ] {
        let output = fixture.run_raw(document);
        assert!(
            !output.status.success(),
            "invalid batch unexpectedly passed"
        );
    }
    let missing = BatchFixture::new();
    std::fs::remove_file(missing.plugin.join(RESET_PATHS[4])).expect("remove reset material");
    assert!(
        !missing.run_raw(cases_document(&CASES)).status.success(),
        "missing reset material unexpectedly passed"
    );
}

struct BatchFixture {
    _root: TempDir,
    plugin: PathBuf,
    snapshots: Vec<(PathBuf, Vec<u8>)>,
}

impl BatchFixture {
    fn new() -> Self {
        let root = tempfile::tempdir().expect("candidate projection root");
        let plugin = complete_plugin_fixture(root.path()).expect("candidate plugin fixture");
        make_candidate_proven_windows_package(&plugin);
        let snapshots = RESET_PATHS
            .iter()
            .map(|relative| {
                let path = plugin.join(relative);
                let contents = std::fs::read(&path).expect("candidate reset material");
                (path, contents)
            })
            .collect();
        Self {
            _root: root,
            plugin,
            snapshots,
        }
    }

    fn run(&self, document: Value) -> Value {
        let output = self.run_raw(document);
        assert!(
            output.status.success(),
            "candidate batch failed:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice(&output.stdout).expect("candidate batch JSON")
    }

    fn run_raw(&self, document: Value) -> std::process::Output {
        std::fs::write(
            self.plugin.join("source-projection-batch.json"),
            serde_json::to_vec(&document).expect("candidate batch JSON"),
        )
        .expect("candidate batch input");
        let mut command = FixtureCommand::new("python3");
        command
            .arg(
                Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("scripts/inspect-release-archive-contract.py"),
            )
            .arg("source-projection-batch");
        command
            .arg_path(&self.plugin)
            .output()
            .expect("candidate batch should start")
    }

    fn reset(&self) {
        for (path, contents) in &self.snapshots {
            std::fs::write(path, contents).expect("reset candidate projection");
        }
    }

    fn assert_reset(&self) {
        for (path, contents) in &self.snapshots {
            assert_eq!(
                std::fs::read(path).expect("candidate reset material"),
                *contents
            );
        }
    }
}

fn cases_document(cases: &[(&str, bool)]) -> Value {
    batch_document(cases, json!(RESET_PATHS), cases.len())
}

fn batch_document(cases: &[(&str, bool)], reset_paths: Value, expected_case_count: usize) -> Value {
    json!({
        "expectedCaseCount": expected_case_count,
        "resetPaths": reset_paths,
        "cases": cases.iter().enumerate().map(|(index, (append, _))| json!({
            "id": format!("case-{index}"),
            "append": append,
        })).collect::<Vec<_>>(),
    })
}

fn assert_projection_results(results: &Value) {
    let results = results.as_array().expect("candidate batch results");
    assert_eq!(
        results.len(),
        CASES.len(),
        "incomplete candidate batch results"
    );
    for (index, ((line, succeeds), result)) in CASES.iter().zip(results).enumerate() {
        assert_eq!(result["id"], format!("case-{index}"), "{line}");
        assert_eq!(result["success"], *succeeds, "{line}");
        assert_eq!(result["diagnostic"].is_null(), *succeeds, "{line}");
    }
}

fn assert_direct_cli_parity(
    fixture: &BatchFixture,
    results: &Value,
    rejected_index: usize,
    accepted_index: usize,
) {
    for index in [rejected_index, accepted_index] {
        let (line, succeeds) = CASES[index];
        fixture.reset();
        let wrapper = fixture.plugin.join("mcp/codexy-mcp-lsp");
        let text = std::fs::read_to_string(&wrapper).expect("candidate wrapper");
        std::fs::write(&wrapper, format!("{text}\n{line}\n")).expect("wrapper mutation");
        let output = run_source_projection(&fixture.plugin);
        assert_eq!(output.status.success(), succeeds, "direct CLI {line}");
        let batch = results[index]
            .get("diagnostic")
            .and_then(Value::as_str)
            .unwrap_or_default();
        assert_eq!(
            String::from_utf8_lossy(&output.stderr).trim(),
            batch,
            "diagnostic {line}"
        );
    }
}
