use serde::Deserialize;

const FIXTURE: &str = include_str!("fixtures/session-audit/read-batch-scorecard.json");

#[derive(Debug, Deserialize)]
struct Scorecard {
    sequential: Measurement,
    batched: Measurement,
}

#[derive(Debug, Deserialize)]
struct Measurement {
    input_tokens: u64,
    output_bytes: u64,
    turns: u64,
}

#[test]
fn scorecard_records_turns_without_claiming_output_byte_savings() {
    let scorecard: Scorecard = serde_json::from_str(FIXTURE).expect("scorecard fixture");
    assert!(scorecard.batched.turns < scorecard.sequential.turns);
    assert!(scorecard.batched.input_tokens < scorecard.sequential.input_tokens);
    assert!(scorecard.batched.output_bytes <= scorecard.sequential.output_bytes);
}
