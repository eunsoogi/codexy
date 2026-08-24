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
