use std::collections::BTreeMap;

use super::Observation;

pub(super) fn viable(observed: &BTreeMap<(&str, &str), &Observation>, effort: &str) -> bool {
    if effort == "high" {
        return false;
    }
    let records = records_for(observed, effort);
    let baseline = records_for(observed, "high");
    let complete = records
        .iter()
        .all(|result| numeric_metrics(result) && result.p0_p1_misses == 0 && result.proof_complete);
    let comparable_baseline = baseline.iter().all(|result| numeric_metrics(result));
    let (Some(candidate_repairs), Some(baseline_repairs)) = (repairs(&records), repairs(&baseline))
    else {
        return false;
    };
    complete
        && comparable_baseline
        && acceptance(&records, false) >= 0.95
        && (acceptance(&records, true) - acceptance(&baseline, true) >= 0.05
            || baseline_repairs > 0 && candidate_repairs as f64 <= baseline_repairs as f64 * 0.8)
        && median(&records, |result| &result.wall_time_ms)
            <= median(&baseline, |result| &result.wall_time_ms) * 1.5
        && median(&records, |result| &result.observed_cost_usd)
            <= median(&baseline, |result| &result.observed_cost_usd) * 1.5
}

fn records_for<'a>(
    observed: &'a BTreeMap<(&str, &str), &'a Observation>,
    effort: &str,
) -> Vec<&'a Observation> {
    observed
        .values()
        .copied()
        .filter(|result| result.thinking == effort)
        .collect()
}

fn numeric_metrics(result: &Observation) -> bool {
    result.tokens.is_number()
        && result.wall_time_ms.is_number()
        && result.observed_cost_usd.is_number()
}

fn acceptance(records: &[&Observation], first_pass: bool) -> f64 {
    records
        .iter()
        .filter(|result| {
            result.acceptance == "pass" && (!first_pass || result.repairs_retries == 0)
        })
        .count() as f64
        / records.len() as f64
}

fn repairs(records: &[&Observation]) -> Option<u64> {
    records.iter().try_fold(0_u64, |total, result| {
        total.checked_add(u64::from(result.repairs_retries))
    })
}

fn median(records: &[&Observation], field: impl Fn(&Observation) -> &serde_json::Value) -> f64 {
    let mut values = records
        .iter()
        .filter_map(|result| field(result).as_f64())
        .collect::<Vec<_>>();
    values.sort_by(f64::total_cmp);
    values[values.len() / 2]
}
