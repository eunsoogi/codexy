use std::collections::{HashMap, HashSet};

use anyhow::{Context, Result, ensure};

use super::{Case, ensure_field_order, record};

// Preserve digest contract data without duplicating the corpus' prose examples.
fn expected_removed_hashes(case_id: &str) -> &'static str {
    match case_id {
        "AR-P01" => "4698100c44e18ce002fb52e544c4f2f7116c7c4bde4f8583770a3d539551adaf=conflict",
        "AR-P02" => "ace4053882093d9aed83154b6213b3b092f0c43f93bb8525151794d643d9ef4f=superseded",
        "AR-P03" => "07fb572ae4e871377b77bbb71fc1196d9acd2e37919cc3232141d4a1fd478486=duplicate",
        "AR-P04" => "b25a237cda0ead7ea34226c04f030eabdc9e4837f3de29a6fff77859221076a0=duplicate",
        "AR-P05" => "d2939e8b3fb720381b3f8db82e967f837aed91a827035c1259f9b409df2d2847=conflict",
        "AR-P06" => "679232fc31a552f355ae28833f45361c94582a2bde20199902398c333c5e17a8=superseded",
        "AR-P07" => "0bdfd49beddc592c0cfb3a8a495af6cfd21e75454ded357572689b6a0779ac56=conflict",
        "AR-P08" => "bf6a1c1f703f98b2a7d489b8f3069686dd6a9734baf9f9457f6de2dcf0238233=conflict",
        "AR-P09" => {
            "57fd9de1296d67ed2f33ce1c1de1d0652b53865a7418d943e3b3eda12c28eed2=duplicate, c6a6edba7d5a5310f0a3a3ca5f7474750040d4f1004cd29a8b073984c698320a=duplicate"
        }
        _ => "<empty>",
    }
}

// The prune corpus exercises receipt outcomes, stable digests, and mutation-free handoffs.
pub(super) fn check_prune_artifact_claims(cases: &[Case]) -> Result<()> {
    let handoff_reasons = [
        "CODE_BEHAVIOR",
        "MULTIPLE_ARTIFACTS",
        "CANONICAL_MOVEMENT",
        "AMBIGUOUS_AUTHORITY",
    ];
    let removal_kinds = ["conflict", "superseded", "duplicate"];
    let mut digest_kinds = HashMap::new();
    let mut observed_handoffs = HashSet::new();
    let mut observed_removals = HashSet::new();
    let mut observed_outcomes = HashSet::new();

    for case in cases {
        ensure_field_order(case, &["input", "result"])?;
        let (input_keys, input) = record(case, "input")?;
        let (result_keys, result) = record(case, "result")?;
        ensure!(
            input_keys.iter().map(String::as_str).eq([
                "scenario",
                "request",
                "artifact",
                "governing_source",
                "artifact_id",
                "governing_source_id",
            ]),
            "input record fields changed in {}",
            case.id
        );
        ensure!(
            result_keys.iter().map(String::as_str).eq([
                "expected_artifact",
                "expected_outcome",
                "removed",
                "handoff_reason",
            ]),
            "result record fields changed in {}",
            case.id
        );
        for key in ["scenario", "request", "artifact", "governing_source"] {
            ensure!(!input[key].is_empty(), "missing {key} in {}", case.id);
        }
        ensure!(!input["artifact_id"].is_empty());
        ensure!(!input["governing_source_id"].is_empty());

        let outcome = result.get("expected_outcome").context("missing outcome")?;
        let expected_artifact = result
            .get("expected_artifact")
            .context("missing artifact")?;
        let removed = result.get("removed").context("missing removals")?;
        let reason = result
            .get("handoff_reason")
            .context("missing handoff reason")?;

        if case.classification == "POSITIVE" {
            let expected = if input["artifact"] == input["governing_source"] {
                "NO_CHANGE"
            } else {
                "UPDATED"
            };
            ensure!(*outcome == expected, "unexpected outcome in {}", case.id);
            ensure!(*reason == "NONE", "unexpected handoff in {}", case.id);
            if *outcome == "NO_CHANGE" {
                ensure!(*expected_artifact == input["artifact"]);
                ensure!(*removed == "<empty>");
            } else {
                ensure!(*expected_artifact != input["artifact"]);
                ensure!(*removed != "<empty>");
            }
            observed_outcomes.insert((*outcome).to_owned());
        } else {
            ensure!(
                *outcome == "HANDOFF_REQUIRED",
                "unexpected handoff in {}",
                case.id
            );
            ensure!(
                *expected_artifact == "<empty>",
                "handoff mutates in {}",
                case.id
            );
            ensure!(*removed == "<empty>", "handoff mutates in {}", case.id);
            ensure!(
                handoff_reasons.contains(&reason.as_str()),
                "unknown handoff in {}",
                case.id
            );
            observed_handoffs.insert((*reason).to_owned());
        }

        let expected_removed = expected_removed_hashes(&case.id);
        ensure!(
            *removed == expected_removed,
            "removed hashes changed in {}",
            case.id
        );
        if *removed != "<empty>" {
            for entry in removed.split(", ") {
                let (digest, kind) = entry
                    .split_once('=')
                    .ok_or_else(|| anyhow::anyhow!("invalid removal entry in {}", case.id))?;
                ensure!(
                    digest.len() == 64
                        && digest
                            .bytes()
                            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase()),
                    "invalid removal digest in {}",
                    case.id
                );
                ensure!(
                    removal_kinds.contains(&kind),
                    "unknown removal kind in {}",
                    case.id
                );
                observed_removals.insert(kind.to_owned());
                if let Some(previous) = digest_kinds.insert(digest.to_owned(), kind.to_owned()) {
                    ensure!(
                        previous == kind,
                        "digest changes removal kind in {}",
                        case.id
                    );
                }
            }
        }
    }

    ensure!(observed_handoffs.len() == handoff_reasons.len());
    ensure!(observed_removals.len() == removal_kinds.len());
    ensure!(observed_outcomes.len() == 2);
    Ok(())
}
