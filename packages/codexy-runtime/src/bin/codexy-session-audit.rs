use std::{fs, io::Read as _, path::PathBuf};

use anyhow::{Context as _, Result, bail};
use clap::{ArgGroup, Parser};

#[path = "codexy-session-audit/audit_math.rs"]
mod audit_math;
#[path = "codexy-session-audit/codex_session.rs"]
mod codex_session;
#[path = "codexy-session-audit/generic_session.rs"]
mod generic_session;
#[path = "codexy-session-audit/receipt.rs"]
mod receipt;
#[path = "codexy-session-audit/report.rs"]
mod report;
#[path = "codexy-session-audit/scorecard.rs"]
mod scorecard;
#[path = "codexy-session-audit/stage_budget.rs"]
mod stage_budget;

use report::{Report, SessionReport};
use scorecard::schema::{Availability, Candidate, Comparison, MeasureAvailability, Thresholds};

pub(crate) const MAX_INPUT_BYTES: usize = 8 * 1024 * 1024;

#[derive(Debug, Parser)]
#[command(about = "Report bounded, metadata-only Codex session aggregates.")]
#[command(group(
    ArgGroup::new("source")
        .required(true)
        .multiple(false)
        .args(["input", "receipt", "scorecard", "stage_budget"])
))]
struct Cli {
    #[arg(long)]
    input: Option<PathBuf>,
    #[arg(long)]
    receipt: Option<PathBuf>,
    #[arg(long)]
    scorecard: Option<PathBuf>,
    #[arg(long = "stage-budget")]
    stage_budget: Option<PathBuf>,
    #[arg(long, requires = "input")]
    recent_turns: Option<usize>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    if let Some(receipt) = cli.receipt {
        let result = receipt::validate_file(&receipt)?;
        println!("{}", serde_json::to_string_pretty(&result)?);
        return Ok(());
    }
    if let Some(scorecard) = cli.scorecard {
        let result = scorecard::validate_file(&scorecard)?;
        println!("{}", serde_json::to_string_pretty(&result)?);
        return Ok(());
    }
    if let Some(stage_budget) = cli.stage_budget {
        let result = stage_budget::validate_file(&stage_budget)?;
        println!("{}", serde_json::to_string_pretty(&result)?);
        return Ok(());
    }
    let recent_turns = cli.recent_turns.unwrap_or(3);
    let input_path = cli.input.context("--input is required")?;
    if recent_turns == 0 {
        bail!("--recent-turns must be at least 1");
    }
    let input_file = fs::File::open(&input_path)
        .with_context(|| format!("opening session metadata input {}", input_path.display()))?;
    let mut input_bytes = Vec::new();
    input_file
        .take((MAX_INPUT_BYTES + 1) as u64)
        .read_to_end(&mut input_bytes)
        .with_context(|| format!("reading session metadata input {}", input_path.display()))?;
    if input_bytes.len() > MAX_INPUT_BYTES {
        bail!("session metadata input exceeds {MAX_INPUT_BYTES} bytes");
    }
    let input = String::from_utf8(input_bytes)
        .with_context(|| format!("decoding session metadata input {}", input_path.display()))?;
    let report = match stage_budget::detect_input_format(&input)? {
        stage_budget::InputFormat::Codex => codex_session::audit(&input, recent_turns)?,
        stage_budget::InputFormat::Generic => generic_session::audit(&input, recent_turns)?,
    };
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

fn is_safe_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
}

#[allow(clippy::cast_precision_loss)]
fn validate_scorecard_outcomes(
    comparisons: &[&Comparison],
    availability: &MeasureAvailability,
    thresholds: &Thresholds,
) -> Result<()> {
    if availability.input_tokens != Availability::Available
        || availability.tool_output_bytes != Availability::Available
    {
        bail!("observable decisions require input-token and tool-output measures");
    }
    let mut input_reductions = comparisons
        .iter()
        .map(|comparison| {
            reduction(
                comparison.before.input_tokens,
                comparison.after.input_tokens,
            )
        })
        .collect::<Result<Vec<_>>>()?;
    input_reductions.sort_by(f64::total_cmp);
    let median = input_reductions[input_reductions.len() / 2];
    let before_tool = percentile95(
        comparisons
            .iter()
            .map(|comparison| comparison.before.tool_output_bytes)
            .collect::<Option<Vec<_>>>()
            .context("observable decisions require complete metric pairs")?,
    );
    let after_tool = percentile95(
        comparisons
            .iter()
            .map(|comparison| comparison.after.tool_output_bytes)
            .collect::<Option<Vec<_>>>()
            .context("observable decisions require complete metric pairs")?,
    );
    let tool_reduction = reduction(Some(before_tool), Some(after_tool))?;
    let totals = |after: bool| {
        comparisons.iter().fold(
            (0_u128, 0_u128, 0_u128, 0_u128, 0_u128, 0_u128),
            |total, comparison| {
                let value = if after {
                    &comparison.after
                } else {
                    &comparison.before
                };
                (
                    total.0 + u128::from(value.acceptance_runs),
                    total.1 + u128::from(value.accepted_runs),
                    total.2 + u128::from(value.p0_p1_misses),
                    total.3 + u128::from(value.proof_complete_runs),
                    total.4 + u128::from(value.repairs),
                    total.5 + u128::from(value.review_cycles),
                )
            },
        )
    };
    let before = totals(false);
    let after = totals(true);
    let acceptance = 100.0 * after.1 as f64 / after.0 as f64;
    let repair_drop = u128::from(thresholds.max_repair_cycle_increase.unsigned_abs());
    let review_drop = u128::from(thresholds.max_review_cycle_increase.unsigned_abs());
    if median < thresholds.median_input_token_reduction_min_pct
        || tool_reduction < thresholds.p95_tool_output_byte_reduction_min_pct
        || after.2 > u128::from(thresholds.max_p0_p1_misses)
        || acceptance < thresholds.acceptance_min_pct
        || after.3 != after.0
        || after.4 + repair_drop > before.4
        || after.5 + review_drop > before.5
    {
        bail!("observable decision outcomes must satisfy every scorecard threshold");
    }
    Ok(())
}

fn validate_scorecard_candidate(candidate: &Candidate) -> Result<()> {
    if candidate.head.len() != 40
        || !candidate.head.bytes().all(|byte| byte.is_ascii_hexdigit())
        || candidate
            .installed_content_sha256
            .as_ref()
            .is_some_and(|digest| {
                digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit())
            })
    {
        bail!("scorecard candidate must use a 40-character head and a valid available digest");
    }
    Ok(())
}

fn validate_scorecard_thresholds(thresholds: &Thresholds) -> Result<()> {
    if thresholds.median_input_token_reduction_min_pct < 25.0
        || thresholds.p95_tool_output_byte_reduction_min_pct < 40.0
        || thresholds.max_p0_p1_misses != 0
        || thresholds.acceptance_min_pct < 95.0
        || thresholds.max_repair_cycle_increase > 0
        || thresholds.max_review_cycle_increase > 0
    {
        bail!("scorecard thresholds must preserve the packaged acceptance floors");
    }
    Ok(())
}

const fn availability_pairs(
    availability: &MeasureAvailability,
) -> [(&'static str, Availability); 6] {
    [
        ("inputTokens", availability.input_tokens),
        ("wallTimeMs", availability.wall_time_ms),
        ("observedCostUsd", availability.observed_cost_usd),
        ("toolInputBytes", availability.tool_input_bytes),
        ("toolOutputBytes", availability.tool_output_bytes),
        ("cacheInputTokens", availability.cache_input_tokens),
    ]
}

fn percentile95(mut values: Vec<u64>) -> u64 {
    values.sort_unstable();
    let index = (values.len() * 95).div_ceil(100).saturating_sub(1);
    values.get(index).copied().unwrap_or_default()
}

#[allow(clippy::cast_precision_loss)]
fn reduction(before: Option<u64>, after: Option<u64>) -> Result<f64> {
    let before = before.context("observable decisions require complete metric pairs")?;
    let after = after.context("observable decisions require complete metric pairs")?;
    if before == 0 {
        bail!("observable reduction baselines must be positive");
    }
    Ok(100.0 * (before as f64 - after as f64) / before as f64)
}
