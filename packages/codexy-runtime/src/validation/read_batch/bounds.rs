use serde_json::{Number, Value};

#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq)]
pub enum ScalarField {
    OperationsOutputBound,
    AggregateOutputBound,
    OutcomesOutputBytes,
    OutcomesAttempts,
    MeasurementsInputTokensValue,
    MeasurementsOutputBytes,
}

impl ScalarField {
    pub const ALL: [Self; 6] = [
        Self::OperationsOutputBound,
        Self::AggregateOutputBound,
        Self::OutcomesOutputBytes,
        Self::OutcomesAttempts,
        Self::MeasurementsInputTokensValue,
        Self::MeasurementsOutputBytes,
    ];

    #[must_use]
    pub const fn path(self) -> &'static str {
        match self {
            Self::OperationsOutputBound => "operations[].outputBound",
            Self::AggregateOutputBound => "aggregateOutputBound",
            Self::OutcomesOutputBytes => "outcomes[].outputBytes",
            Self::OutcomesAttempts => "outcomes[].attempts",
            Self::MeasurementsInputTokensValue => "measurements.inputTokens.value",
            Self::MeasurementsOutputBytes => "measurements.outputBytes",
        }
    }
}

pub fn validate_scalar(field: ScalarField, value: &Value) -> Result<u64, String> {
    let parsed = value
        .as_number()
        .and_then(parse_json_integer)
        .ok_or_else(|| format!("{} must be a non-negative JSON integer", field.path()))?;
    if matches!(field, ScalarField::OutcomesAttempts) && parsed == 0 {
        return Err(format!("{} must be at least one", field.path()));
    }
    Ok(parsed)
}

fn parse_json_integer(number: &Number) -> Option<u64> {
    let text = number.to_string();
    let (mantissa, exponent) = if let Some((mantissa, exponent)) = text.split_once(['e', 'E']) {
        (mantissa, exponent.parse::<i32>().ok()?)
    } else {
        (text.as_str(), 0)
    };
    let (negative, mantissa) = mantissa
        .strip_prefix('-')
        .map_or((false, mantissa), |mantissa| (true, mantissa));
    let (whole, fraction) = mantissa
        .split_once('.')
        .map_or((mantissa, ""), |parts| parts);
    let mut digits = format!("{whole}{fraction}");
    digits = digits.trim_start_matches('0').to_owned();
    if digits.is_empty() {
        return Some(0);
    }
    let fractional_digits = i32::try_from(fraction.len()).ok()?;
    let scale = exponent.checked_sub(fractional_digits)?;
    if scale >= 0 {
        let zeros = usize::try_from(scale).ok()?;
        if digits.len().checked_add(zeros)? > 20 {
            return None;
        }
        digits.push_str(&"0".repeat(zeros));
    } else {
        let remove = usize::try_from(scale.unsigned_abs()).ok()?;
        if remove > digits.len()
            || !digits[digits.len() - remove..]
                .chars()
                .all(|character| character == '0')
        {
            return None;
        }
        digits.truncate(digits.len() - remove);
        digits = digits.trim_start_matches('0').to_owned();
    }
    if negative || digits.len() > 20 {
        return None;
    }
    digits.parse::<u64>().ok()
}
