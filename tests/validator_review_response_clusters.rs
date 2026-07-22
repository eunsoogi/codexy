use codexy_runtime::validation::review_response_cluster_diagnostics;

use crate::support::{self, copy_plugin_fixture, stderr, TestResult};

const REQUIRED_CONTRACTS: &[(&str, &str)] = &[
    (
        "skills/codex-orchestration/SKILL.md",
        "Before review-response edits, MUST create one root-cause cluster for each actionable defect class.",
    ),
    (
        "skills/git-workflow/SKILL.md",
        "Review-response handoffs MUST include a typed `ReviewClusterReceipt` for every actionable defect class before implementation edits begin.",
    ),
    (
        "skills/proof-driven-completion/SKILL.md",
        "Review-response proof MUST exercise a bounded positive and negative matrix for every repaired root-cause cluster.",
    ),
    (
        "agents/codexy-sentinel.toml",
        "Sentinel MUST consolidate examples from the same defect class into one blocker with one structural repair strategy.",
    ),
];

#[test]
fn instruction_policy_requires_review_cluster_contract_on_every_surface() -> TestResult {
    for (relative, clause) in REQUIRED_CONTRACTS {
        let (_temp, plugin_root) = copy_plugin_fixture()?;
        let path = plugin_root.join(relative);
        let text = std::fs::read_to_string(&path)?;
        std::fs::write(&path, text.replace(clause, "removed root-cause contract."))?;

        let output = support::validator_instruction_policy(&plugin_root)?;
        assert!(!output.status.success(), "{relative} was not enforced");
        assert!(stderr(&output).contains("root-cause review cluster"));
    }
    Ok(())
}

#[test]
fn review_cluster_harness_rejects_case_specific_classification_reply() {
    let errors = review_response_cluster_diagnostics(
        r#"{
          "state":"repaired",
          "clusters":[{
            "defect_class":"classification-boundary",
            "violated_invariant":"owners are classified from authoritative metadata",
            "structural_boundary":"metadata parser",
            "threads":["PRRT_classification_one","PRRT_classification_two"],
            "matrix":{"positive":["canonical metadata"],"negative":["GFM owner table"]},
            "repair":{"kind":"case_exception","quoted_input":"quoted owner table"}
          }]
        }"#,
    );

    assert!(
        errors.iter().any(|error| error.contains("structural repair")),
        "case-specific repair unexpectedly passed: {errors:?}"
    );
}

#[test]
fn review_cluster_harness_accepts_structural_clusters_and_distinct_reopen() {
    let errors = review_response_cluster_diagnostics(
        r#"{
          "state":"reopened",
          "clusters":[
            {
              "defect_class":"classification-boundary",
              "violated_invariant":"owners are classified from authoritative metadata",
              "structural_boundary":"metadata parser",
              "threads":["PRRT_classification_one","PRRT_classification_two"],
              "matrix":{"positive":["canonical metadata"],"negative":["GFM owner table"]},
              "repair":{"kind":"structural","boundary":"metadata parser","strategy":"authoritative metadata classifier","removed_case_specific_behavior":true}
            },
            {
              "defect_class":"command-normalization",
              "violated_invariant":"repository identity survives wrapper composition",
              "structural_boundary":"command normalization",
              "threads":["PRRT_command_one"],
              "matrix":{"positive":["relative GIT_DIR"],"negative":["foreign repository"]},
              "repair":{"kind":"structural","boundary":"command normalization","strategy":"canonical repository resolver","removed_case_specific_behavior":true},
              "reopen":{"kind":"distinct_invariant","invariant":"environment ownership is preserved"}
            }
          ]
        }"#,
    );

    assert!(errors.is_empty(), "structural clusters failed: {errors:?}");
}

#[test]
fn review_cluster_harness_observes_planned_to_repaired_transition() {
    let planned = r#"{"state":"planned","clusters":[{"defect_class":"classification-boundary","violated_invariant":"authoritative metadata wins","structural_boundary":"metadata parser","threads":["PRRT_classification_one"],"matrix":{"positive":["canonical metadata"],"negative":["GFM owner table"]}}]}"#;
    let repaired = r#"{"state":"repaired","clusters":[{"defect_class":"classification-boundary","violated_invariant":"authoritative metadata wins","structural_boundary":"metadata parser","threads":["PRRT_classification_one"],"matrix":{"positive":["canonical metadata"],"negative":["GFM owner table"]},"repair":{"kind":"structural","boundary":"metadata parser","strategy":"classify authoritative metadata once","removed_case_specific_behavior":true}}]}"#;

    assert!(review_response_cluster_diagnostics(planned).is_empty());
    assert!(review_response_cluster_diagnostics(repaired).is_empty());
}

#[test]
fn review_cluster_harness_rejects_phrase_only_and_same_class_reopen() {
    for receipt in [
        r#"{"state":"repaired","clusters":[{"defect_class":"classification","violated_invariant":"same","structural_boundary":"parser","threads":["PRRT_one"],"matrix":{"positive":["ok"],"negative":["no"]},"repair":{"kind":"structural","boundary":"parser","strategy":"same phrase","removed_case_specific_behavior":false}}]}"#,
        r#"{"state":"reopened","clusters":[{"defect_class":"command-normalization","violated_invariant":"repository identity","structural_boundary":"normalizer","threads":["PRRT_one"],"matrix":{"positive":["ok"],"negative":["no"]},"repair":{"kind":"structural","boundary":"normalizer","strategy":"repository resolver","removed_case_specific_behavior":true},"reopen":{"kind":"distinct_invariant","invariant":"repository identity"}}]}"#,
        "not structured evidence",
    ] {
        assert!(
            !review_response_cluster_diagnostics(receipt).is_empty(),
            "invalid receipt unexpectedly passed: {receipt}"
        );
    }
}
