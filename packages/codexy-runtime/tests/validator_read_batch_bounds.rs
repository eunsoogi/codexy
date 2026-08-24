use codexy_runtime::validation::read_batch::bounds::ScalarField;
use serde_json::{Value, json};

const SCHEMA: &str =
    include_str!("../../../plugins/codexy/skills/orchestration/references/read-batch.schema.json");

fn schema_constraint(field: ScalarField) -> Value {
    let schema: Value = serde_json::from_str(SCHEMA).expect("read-batch schema");
    let pointer = match field {
        ScalarField::OperationsOutputBound => "/properties/operations/items/properties/outputBound",
        ScalarField::AggregateOutputBound => "/properties/aggregateOutputBound",
        ScalarField::OutcomesOutputBytes => "/properties/outcomes/items/properties/outputBytes",
        ScalarField::OutcomesAttempts => "/properties/outcomes/items/properties/attempts",
        ScalarField::MeasurementsInputTokensValue => {
            "/properties/measurements/properties/inputTokens/properties/value"
        }
        ScalarField::MeasurementsOutputBytes => "/properties/measurements/properties/outputBytes",
    };
    schema
        .pointer(pointer)
        .cloned()
        .expect("scalar schema property")
}

fn integral_number(value: &Value) -> Option<i128> {
    let number = value.as_number()?;
    let text = number.to_string();
    let (mantissa, exponent) = text
        .split_once(['e', 'E'])
        .map_or((text.as_str(), 0_i32), |(mantissa, exponent)| {
            (mantissa, exponent.parse::<i32>().ok().unwrap_or(i32::MAX))
        });
    let (negative, mantissa) = mantissa
        .strip_prefix('-')
        .map_or((false, mantissa), |mantissa| (true, mantissa));
    let (whole, fraction) = mantissa
        .split_once('.')
        .map_or((mantissa, ""), |parts| parts);
    let mut digits = format!("{whole}{fraction}");
    let scale = exponent.checked_sub(i32::try_from(fraction.len()).ok()?)?;
    if scale >= 0 {
        digits.push_str(&"0".repeat(usize::try_from(scale).ok()?));
    } else {
        let remove = usize::try_from(scale.unsigned_abs()).ok()?;
        if remove > digits.len()
            || !digits[digits.len().saturating_sub(remove)..]
                .chars()
                .all(|character| character == '0')
        {
            return None;
        }
        digits.truncate(digits.len().saturating_sub(remove));
    }
    let digits = digits.trim_start_matches('0');
    let magnitude = if digits.is_empty() {
        0
    } else {
        digits.parse::<i128>().ok()?
    };
    Some(if negative { -magnitude } else { magnitude })
}

fn schema_accepts(field: ScalarField, value: &Value) -> bool {
    let constraint = schema_constraint(field);
    constraint["type"] == "integer"
        && integral_number(value).is_some_and(|number| {
            constraint_integer(&constraint, "minimum").is_none_or(|minimum| number >= minimum)
                && constraint_integer(&constraint, "maximum")
                    .is_none_or(|maximum| number <= maximum)
        })
}

fn constraint_integer(constraint: &Value, name: &str) -> Option<i128> {
    constraint.get(name).and_then(integral_number)
}

#[test]
fn baseline_schema_and_runtime_agree_on_small_positive_u64_values() {
    for field in ScalarField::ALL {
        let value = json!(7);
        assert!(schema_accepts(field, &value), "{}", field.path());
        assert!(
            codexy_runtime::validation::read_batch::bounds::validate_scalar(field, &value).is_ok(),
            "{}",
            field.path()
        );
    }
}

#[derive(Debug)]
struct ScalarCase {
    label: &'static str,
    value: Value,
    expected: bool,
}

fn scalar_cases(field: ScalarField) -> Vec<ScalarCase> {
    let zero_expected = !matches!(
        field,
        ScalarField::OperationsOutputBound | ScalarField::OutcomesAttempts
    );
    let positive_label = if matches!(field, ScalarField::OutcomesAttempts) {
        "attempts=1"
    } else {
        "positive"
    };
    let positive_value = if matches!(field, ScalarField::OutcomesAttempts) {
        json!(1)
    } else {
        json!(7)
    };
    let zero_label = if matches!(field, ScalarField::OutcomesAttempts) {
        "attempts=0"
    } else {
        "zero"
    };
    vec![
        ScalarCase {
            label: positive_label,
            value: positive_value,
            expected: true,
        },
        ScalarCase {
            label: "integral-1.0",
            value: json!(1.0),
            expected: true,
        },
        ScalarCase {
            label: "non-integral",
            value: json!(1.5),
            expected: false,
        },
        ScalarCase {
            label: "negative",
            value: json!(-1),
            expected: false,
        },
        ScalarCase {
            label: "u64::MAX",
            value: json!(u64::MAX),
            expected: true,
        },
        ScalarCase {
            label: "u64::MAX+1",
            value: serde_json::from_str("18446744073709551616").expect("u64 overflow JSON"),
            expected: false,
        },
        ScalarCase {
            label: zero_label,
            value: json!(0),
            expected: zero_expected,
        },
    ]
}

#[test]
fn scalar_matrix_is_field_specific_across_schema_and_runtime() {
    let mut failures = Vec::new();
    for field in ScalarField::ALL {
        for case in std::iter::repeat_n(field, 1).flat_map(scalar_cases) {
            let schema_ok = schema_accepts(field, &case.value);
            let runtime_ok =
                codexy_runtime::validation::read_batch::bounds::validate_scalar(field, &case.value)
                    .is_ok();
            if schema_ok != case.expected || runtime_ok != case.expected {
                failures.push(format!(
                    "{} [{}]: schema={schema_ok} runtime={runtime_ok} expected={}",
                    field.path(),
                    case.label,
                    case.expected
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "scalar parity matrix RED:\n{}",
        failures.join("\n")
    );
}
