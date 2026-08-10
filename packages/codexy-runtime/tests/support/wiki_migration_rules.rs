use super::{
    wiki_active_token_stream::{Clause, Mode, clauses},
    wiki_minimal_contract_markdown::Document,
};

pub(crate) fn validate_migration_rules(source: &str) -> Result<(), String> {
    let document = Document::parse(source)?;
    let scope = Rules::new(clauses(&document, &document.section("## Scope")?));
    let procedure = Rules::new(clauses(&document, &document.section("## Procedure")?));
    scope.require(
        Mode::Must,
        &["preserve", "existing"],
        &["raw/", "wiki/", "_index.md", "log.md"],
    )?;
    scope.require(
        Mode::MustNot,
        &["delete", "overwrite", "rename", "existing topic data"],
        &[],
    )?;
    procedure.require_ordered(&[
        (
            Mode::Must,
            &[
                "validate",
                "referenced provenance",
                "freshness input",
                "before any log",
                "derived write",
            ],
            &[],
        ),
        (
            Mode::Must,
            &["preserve", "complete relative", "scalar", "exactly"],
            &["sources:"],
        ),
        (Mode::Must, &["stop"], &[]),
        (Mode::Must, &["report", "provenance gap"], &[]),
        (
            Mode::Must,
            &[
                "leave the entire topic tree unchanged",
                "provenance failure",
            ],
            &[],
        ),
        (
            Mode::Must,
            &["stage", "all derived changes", "completion log entry"],
            &[],
        ),
        (
            Mode::Must,
            &[
                "validate",
                "staged derived changes",
                "completion log entry",
                "together",
            ],
            &[],
        ),
        (Mode::Must, &["atomically commit derived files"], &[]),
        (
            Mode::Must,
            &["append one migration entry", "final commit action"],
            &[],
        ),
        (
            Mode::Must,
            &[
                "roll back every staged",
                "derived change",
                "leave the entire topic tree unchanged",
            ],
            &[],
        ),
    ])?;
    procedure.require_only_final_log_append()
}

struct Rules {
    clauses: Vec<Clause>,
}

impl Rules {
    fn new(clauses: Vec<Clause>) -> Self {
        Self { clauses }
    }

    fn require(&self, mode: Mode, terms: &[&str], inline: &[&str]) -> Result<usize, String> {
        let matches = self
            .clauses
            .iter()
            .enumerate()
            .filter(|(_, clause)| matches(clause, terms, inline))
            .collect::<Vec<_>>();
        if matches.len() != 1 || matches[0].1.mode != mode {
            return Err(format!(
                "missing, duplicate, or contradictory typed normative clause: {}",
                terms.join(", ")
            ));
        }
        reject_qualifiers(matches[0].1)?;
        Ok(matches[0].0)
    }

    fn require_ordered(&self, expected: &[(Mode, &[&str], &[&str])]) -> Result<(), String> {
        let mut previous = 0;
        for (mode, terms, inline) in expected {
            let found = self.require(*mode, terms, inline)?;
            if found < previous {
                return Err("reordered typed normative clause".into());
            }
            previous = found + 1;
        }
        Ok(())
    }

    fn require_only_final_log_append(&self) -> Result<(), String> {
        let appends = self
            .clauses
            .iter()
            .filter(|clause| {
                phrase(clause, "append")
                    && (clause.inline.iter().any(|identity| identity == "log.md")
                        || clause.contains_plain_identity("log.md"))
            })
            .count();
        (appends == 1)
            .then_some(())
            .ok_or("missing, duplicate, or early log.md append clause".into())
    }
}

fn matches(clause: &Clause, terms: &[&str], inline: &[&str]) -> bool {
    terms.iter().all(|term| phrase(clause, term))
        && (inline.is_empty()
            || (clause.inline.len() == inline.len()
                && inline
                    .iter()
                    .all(|identity| clause.inline.iter().any(|found| found == identity))))
}

fn reject_qualifiers(clause: &Clause) -> Result<(), String> {
    [
        "except",
        "unless",
        "baseline",
        "allowlist",
        "compatibility",
        "alias",
        "external",
        "restore",
    ]
    .iter()
    .all(|term| !clause.prose.iter().any(|word| word == term))
    .then_some(())
    .ok_or_else(|| "qualified typed normative clause".into())
}

fn phrase(clause: &Clause, term: &str) -> bool {
    clause.contains_phrase(term)
}
