//! Requirement-linked hard work routing regressions.

use super::*;

#[test]
fn issue_1423_rejects_positional_hard_work_evidence() -> TestResult {
    // Sentinel's populated sequence must not certify the closed named-facts contract.
    let mut positional = request("general", SOL);
    positional["hard_work"] = json!([
        "coupled_contracts",
        ["parser", "admission", "delivery"],
        "One shared authenticated assignment"
    ]);
    rejects(positional, "hard_work must be a JSON object")
}

#[test]
fn only_evidenced_hard_work_selects_hard_worker() -> TestResult {
    for facts in hard_work_cases()? {
        let mut hard = request("general", HARD_WORKER);
        hard["hard_work"] = facts;
        hard["simple_predicates"]["deterministic_oracle"] = json!(false);
        check(hard.clone(), selected("generic", "create_thread", HARD_WORKER))?;
        hard["codex_thread_capabilities"]["models"]
            .as_array_mut()
            .ok_or("models")?
            .push(json!({"model":DEFAULT_WORKER.0,"thinking":[DEFAULT_WORKER.1]}));
        check(hard.clone(), selected("generic", "create_thread", HARD_WORKER))?;
        hard["codex_thread_capabilities"] =
            request("general", DEFAULT_WORKER)["codex_thread_capabilities"].clone();
        check(hard.clone(), fallback())?;
        // Authenticated follow-up delivery rejects Sol/High just as creation does.
        hard["codex_thread_capabilities"] =
            request("general", SOL)["codex_thread_capabilities"].clone();
        check(hard.clone(), fallback())?;
        for classification in [
            "simple",
            "bounded_evidence",
            "ambiguous",
            "incomplete",
            "high_risk",
        ] {
            let mut other = if classification == "bounded_evidence" {
                bounded_request()
            } else {
                request(classification, DEFAULT_WORKER)
            };
            other["hard_work"] = hard["hard_work"].clone();
            check(
                other,
                if ["simple", "bounded_evidence"].contains(&classification) {
                    selected("generic", "create_thread", DEFAULT_WORKER)
                } else {
                    fallback()
                },
            )?;
        }
        for predicate in [
            "fixed_scope",
            "low_risk_reversible",
            "no_unresolved_decision",
        ] {
            let mut unsettled = request("general", HARD_WORKER);
            unsettled["hard_work"] = hard["hard_work"].clone();
            unsettled["simple_predicates"][predicate] = json!(false);
            check(unsettled, fallback())?;
        }
    }
    let mut ordinary = request("general", DEFAULT_WORKER);
    ordinary["simple_predicates"]["deterministic_oracle"] = json!(false);
    check(ordinary, selected("generic", "create_thread", DEFAULT_WORKER))?;
    let mut available_hard_model_is_not_hard_work = request("general", DEFAULT_WORKER);
    available_hard_model_is_not_hard_work["codex_thread_capabilities"]["models"]
        .as_array_mut()
        .ok_or("models")?
        .push(json!({"model":HARD_WORKER.0,"thinking":[HARD_WORKER.1]}));
    check(
        available_hard_model_is_not_hard_work,
        selected("generic", "create_thread", DEFAULT_WORKER),
    )?;
    let ordinary_without_default_model = request("general", HARD_WORKER);
    check(ordinary_without_default_model, fallback())?;
    for predicate in [
        "fixed_scope",
        "low_risk_reversible",
        "no_unresolved_decision",
    ] {
        let mut ordinary = request("general", DEFAULT_WORKER);
        ordinary["simple_predicates"][predicate] = json!(false);
        check(ordinary, fallback())?;
    }
    for pair in [DEFAULT_WORKER, SOL] {
        let mut incomplete = request("general", pair);
        incomplete
            .as_object_mut()
            .ok_or("request")?
            .remove("simple_predicates");
        check(incomplete, fallback())?;
    }
    Ok(())
}

#[test]
fn issue_1423_hard_work_rejects_unsubstantiated_and_malformed_claims() -> TestResult {
    for facts in hard_work_cases()? {
        let mut hard = request("general", HARD_WORKER);
        hard["hard_work"] = facts.clone();
        for field in facts.as_object().ok_or("facts")?.keys() {
            let mut missing = hard.clone();
            missing["hard_work"]
                .as_object_mut()
                .ok_or("facts")?
                .remove(field);
            rejects(missing, "")?;
            if facts[field].is_array() {
                for value in [
                    json!([]),
                    json!(["same", " same ", "same"]),
                    json!(["", " ", ""]),
                ] {
                    let mut invalid = hard.clone();
                    invalid["hard_work"][field] = value;
                    rejects(
                        invalid,
                        "Hard-work routing requires complete, distinct evidence",
                    )?;
                }
            } else if field != "criterion" {
                let mut invalid = hard.clone();
                invalid["hard_work"][field] = json!(" ");
                rejects(
                    invalid,
                    "Hard-work routing requires complete, distinct evidence",
                )?;
            }
        }
        hard["hard_work"]["authority"] = json!("assumed");
        rejects(hard, "")?;
    }
    for facts in [
        json!({}),
        json!({"criterion":"general_implementation"}),
        json!(true),
        json!([]),
    ] {
        let mut hard = request("general", HARD_WORKER);
        hard["hard_work"] = facts;
        rejects(hard, "")?;
    }
    Ok(())
}
