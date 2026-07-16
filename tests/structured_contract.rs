#![allow(dead_code)]

#[path = "structured_contract/parser.rs"]
mod parser;

use parser::{Block, Clause, canonical, contains_phrase};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Modality {
    Required,
    Prohibited,
    Permitted,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Rule {
    pub(crate) id: &'static str,
    pub(crate) subject: &'static str,
    pub(crate) modality: Modality,
    pub(crate) action: &'static [&'static str],
    pub(crate) scope: &'static [&'static str],
    pub(crate) lifecycle: &'static [&'static str],
    pub(crate) heading: Option<&'static str>,
}

impl Rule {
    pub(crate) const fn new(
        id: &'static str,
        subject: &'static str,
        modality: Modality,
        action: &'static [&'static str],
        scope: &'static [&'static str],
    ) -> Self {
        Self {
            id,
            subject,
            modality,
            action,
            scope,
            lifecycle: &[],
            heading: None,
        }
    }

    pub(crate) const fn under_heading(mut self, heading: &'static str) -> Self {
        self.heading = Some(heading);
        self
    }

    pub(crate) const fn in_lifecycle(mut self, lifecycle: &'static [&'static str]) -> Self {
        self.lifecycle = lifecycle;
        self
    }
}

#[derive(Debug)]
pub(crate) struct Contract {
    blocks: Vec<Block>,
    implicit_subject: Option<String>,
}

#[derive(Debug)]
pub(crate) struct ContractError {
    pub(crate) rule_id: &'static str,
    pub(crate) missing: &'static str,
}

#[derive(Default)]
struct Evidence {
    heading: bool,
    subject: bool,
    modality: bool,
    conditionality: bool,
    negation: bool,
    action: bool,
    scope: bool,
    lifecycle: bool,
}

impl Contract {
    pub(crate) fn markdown(text: &str) -> Self {
        Self::with_subject(text, None)
    }

    pub(crate) fn markdown_for_subject(text: &str, subject: &str) -> Self {
        Self::with_subject(text, Some(canonical(subject)))
    }

    fn with_subject(text: &str, implicit_subject: Option<String>) -> Self {
        Self {
            blocks: parser::blocks(text),
            implicit_subject,
        }
    }

    pub(crate) fn assert_rule(&self, rule: Rule) -> Result<(), ContractError> {
        let mut evidence = Evidence::default();
        for block in self.blocks.iter().filter(|block| block.is_active()) {
            if !block.matches_heading(rule.heading) {
                continue;
            }
            evidence.heading = true;
            for clause in block.clauses() {
                if self.matches(rule, &clause, &mut evidence) {
                    return Ok(());
                }
            }
        }
        Err(ContractError {
            rule_id: rule.id,
            missing: evidence.missing(rule),
        })
    }

    fn matches(&self, rule: Rule, clause: &Clause, evidence: &mut Evidence) -> bool {
        let subject = clause.subject_matches(rule.subject)
            || self
                .implicit_subject
                .as_deref()
                .is_some_and(|subject| contains_phrase(subject, rule.subject));
        let modality = clause.modality == rule.modality;
        let conditionality = !clause.conditional;
        let negation = !clause.inverted;
        let action = clause.tail_has(rule.action);
        let scope = clause.tail_has(rule.scope);
        let lifecycle = clause.tail_has(rule.lifecycle);
        evidence.subject |= subject;
        evidence.modality |= modality;
        evidence.conditionality |= conditionality;
        evidence.negation |= negation;
        evidence.action |= action;
        evidence.scope |= scope;
        evidence.lifecycle |= lifecycle;
        subject && modality && conditionality && negation && action && scope && lifecycle
    }
}

impl Evidence {
    fn missing(&self, rule: Rule) -> &'static str {
        if !self.heading {
            "heading"
        } else if !self.conditionality {
            "conditionality"
        } else if !self.subject {
            "subject"
        } else if !self.modality {
            "modality"
        } else if !self.negation {
            "negation"
        } else if !self.action {
            "action"
        } else if !self.scope {
            "scope"
        } else if !rule.lifecycle.is_empty() && !self.lifecycle {
            "lifecycle"
        } else {
            "co-located semantics"
        }
    }
}

pub(crate) fn assert_rules(contract: &Contract, rules: &[Rule]) {
    for rule in rules {
        contract.assert_rule(*rule).unwrap_or_else(|error| {
            panic!(
                "structured contract {} is missing {}",
                error.rule_id, error.missing
            )
        });
    }
}
