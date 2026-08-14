use std::collections::BTreeSet;

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct IssueContract {
    problem: String,
    scope: String,
    acceptance_criteria: Vec<Criterion>,
    owned_invariant_ids: Vec<String>,
    exclusions: Vec<String>,
    adjacent_dependencies: Vec<String>,
}

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct Criterion {
    id: String,
}

impl IssueContract {
    pub(super) fn authority(&self) -> Result<(BTreeSet<&str>, BTreeSet<&str>)> {
        if self.problem.is_empty() || self.scope.is_empty() {
            bail!("review packet issue contract requires problem and scope");
        }
        let criteria = unique(
            self.acceptance_criteria.iter().map(|item| item.id.as_str()),
            "acceptance criterion",
        )?;
        let invariants = unique(
            self.owned_invariant_ids.iter().map(String::as_str),
            "owned invariant",
        )?;
        for values in [&self.exclusions, &self.adjacent_dependencies] {
            unique(values.iter().map(String::as_str), "issue contract value")?;
        }
        if !strictly_sorted(&self.owned_invariant_ids) {
            bail!("review packet owned invariant identifiers must be canonically sorted");
        }
        Ok((criteria, invariants))
    }

    pub(super) fn digest(&self) -> String {
        format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(self).expect("issue contract is serializable"))
        )
    }
}

fn unique<'a>(values: impl Iterator<Item = &'a str>, label: &str) -> Result<BTreeSet<&'a str>> {
    let values = values.collect::<Vec<_>>();
    let unique = values.iter().copied().collect::<BTreeSet<_>>();
    if values.len() != unique.len() || unique.iter().any(|value| value.is_empty()) {
        bail!("review packet {label} values must be unique and non-empty");
    }
    Ok(unique)
}

fn strictly_sorted(values: &[String]) -> bool {
    values.windows(2).all(|pair| pair[0] < pair[1])
}
