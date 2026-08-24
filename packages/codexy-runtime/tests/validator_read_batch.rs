use codexy_runtime::validation::read_batch::{
    InputTokens, Measurements, OutcomeStatus, ReadBatchPlan, ReadOperation, ReadOutcome,
    ReadReference,
};

fn operation(id: &str, locator: &str, bound: u64) -> ReadOperation {
    ReadOperation {
        id: id.to_owned(),
        reference: ReadReference {
            kind: "file".to_owned(),
            locator: locator.to_owned(),
            read_only: true,
            independent: true,
        },
        output_bound: bound,
    }
}

fn valid_plan() -> ReadBatchPlan {
    ReadBatchPlan {
        operations: vec![operation("b", "b.txt", 20), operation("a", "a.txt", 10)],
        aggregate_output_bound: 30,
        outcomes: vec![ReadOutcome {
            id: "a".to_owned(),
            output_bytes: 8,
            attempts: 1,
            status: OutcomeStatus::Success,
        }],
        measurements: Measurements {
            input_tokens: InputTokens { value: 12 },
            output_bytes: 8,
        },
    }
}

#[test]
fn read_batch_accepts_bounded_independent_reads_and_partial_results() {
    let plan = valid_plan();
    assert_eq!(plan.successful_outcomes().len(), 1);
    assert!(plan.validate().is_ok());
}

#[test]
fn read_batch_deserializes_integral_json_forms_for_all_six_scalar_fields() {
    let fields = [
        "operations[].outputBound",
        "aggregateOutputBound",
        "outcomes[].outputBytes",
        "outcomes[].attempts",
        "measurements.inputTokens.value",
        "measurements.outputBytes",
    ];
    let mut failures = Vec::new();
    for (index, field) in fields.into_iter().enumerate() {
        for token in [
            "1.0",
            "1e0",
            "18446744073709551615.0",
            "18446744073709551615e0",
        ] {
            let mut scalars = ["1"; 6];
            scalars[index] = token;
            if let Err(error) = serde_json::from_str::<ReadBatchPlan>(&plan_json(scalars)) {
                failures.push(format!("{field} [{token}]: {error}"));
            }
        }
        for token in [
            "1.5",
            "-1.0",
            "18446744073709551616.0",
            "18446744073709551616e0",
        ] {
            let mut scalars = ["1"; 6];
            scalars[index] = token;
            if serde_json::from_str::<ReadBatchPlan>(&plan_json(scalars)).is_ok() {
                failures.push(format!("{field} [{token}]: unexpectedly accepted"));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "integral JSON form RED:\n{}",
        failures.join("\n")
    );
}

fn plan_json(scalars: [&str; 6]) -> String {
    format!(
        r#"{{"operations":[{{"id":"a","reference":{{"kind":"file","locator":"a.txt","readOnly":true,"independent":true}},"outputBound":{}}}],"aggregateOutputBound":{},"outcomes":[{{"id":"a","outputBytes":{},"attempts":{},"status":"success"}}],"measurements":{{"inputTokens":{{"value":{}}},"outputBytes":{}}}}}"#,
        scalars[0], scalars[1], scalars[2], scalars[3], scalars[4], scalars[5]
    )
}

#[test]
fn read_batch_rejects_mutations_dependencies_and_missing_bounds() {
    let mut candidate = valid_plan();
    candidate.operations[0].reference.read_only = false;
    assert!(candidate.validate().is_err());

    let mut candidate = valid_plan();
    candidate.operations[0].reference.independent = false;
    assert!(candidate.validate().is_err());

    let mut candidate = valid_plan();
    candidate.operations[0].output_bound = 0;
    assert!(candidate.validate().is_err());
}

#[test]
fn read_batch_rejects_outcome_bytes_above_the_matching_operation_bound() {
    let mut candidate = valid_plan();
    candidate.outcomes[0].output_bytes = 11;
    assert!(candidate.validate().is_err());
}

#[test]
fn read_batch_rejects_zero_outcome_attempts() {
    let mut candidate = valid_plan();
    candidate.outcomes[0].attempts = 0;
    assert!(candidate.validate().is_err());
}
