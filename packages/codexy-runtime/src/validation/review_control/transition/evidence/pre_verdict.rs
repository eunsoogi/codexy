use serde_json::{Value, json};

use super::super::PreVerdictContext;
use super::disposition::{self, Context};

pub(crate) fn check(context: &PreVerdictContext<'_>) -> Result<(), String> {
    let finding_ids = context
        .source
        .get("findings")
        .and_then(Value::as_array)
        .ok_or_else(|| "next-review eligibility source lacks finding coverage".to_owned())?
        .iter()
        .map(|finding| {
            finding
                .get("id")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
                .ok_or_else(|| "next-review eligibility finding id is invalid".to_owned())
        })
        .collect::<Result<Vec<_>, _>>()?;
    let change = json!({
        "from_head": context.from,
        "to_head": context.to,
        "evidence_commit": context.to,
        "finding_ids": finding_ids,
        "finding_disposition": context.source
    });
    let change = change
        .as_object()
        .ok_or_else(|| "next-review eligibility qualifying change is invalid".to_owned())?;
    disposition::check(&Context {
        repository_root: context.repository_root,
        previous_base: context.previous_base,
        current_base: context.current_base,
        current: context.current,
        prior_delta: context.prior_delta,
        change,
        from: context.from,
        evidence: context.to,
    })
}
