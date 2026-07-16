#![allow(dead_code)]

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
            heading: None,
        }
    }

    pub(crate) const fn under_heading(mut self, heading: &'static str) -> Self {
        self.heading = Some(heading);
        self
    }
}

#[derive(Debug)]
pub(crate) struct Contract {
    blocks: Vec<Block>,
}

#[derive(Debug)]
struct Block {
    heading: String,
    text: String,
}

#[derive(Debug)]
pub(crate) struct ContractError {
    pub(crate) rule_id: &'static str,
    pub(crate) missing: &'static str,
}

impl Contract {
    pub(crate) fn markdown(text: &str) -> Self {
        let mut blocks = Vec::new();
        let mut heading = String::new();
        let mut lines = Vec::new();
        for line in text.lines() {
            if let Some(next_heading) = line.trim_start().strip_prefix('#') {
                push_block(&mut blocks, &heading, &lines);
                heading = normalize(next_heading.trim_start_matches('#'));
                lines.clear();
            } else {
                lines.push(line);
            }
        }
        push_block(&mut blocks, &heading, &lines);
        Self { blocks }
    }

    pub(crate) fn assert_rule(&self, rule: Rule) -> Result<(), ContractError> {
        let candidates = self.blocks.iter().filter(|block| {
            !block.heading.contains("historical")
                && rule
                    .heading
                    .is_none_or(|heading| block.heading.contains(&normalize(heading)))
        });
        let mut has_subject = false;
        let mut has_modality = false;
        let mut has_action = false;
        let mut has_scope = false;
        for block in candidates {
            for clause in clauses(&block.text) {
                let subject = clause.contains(&normalize(rule.subject));
                let modality = parse_modality(clause) == Some(rule.modality);
                let action = rule
                    .action
                    .iter()
                    .all(|term| clause.contains(&normalize(term)));
                let scope = rule
                    .scope
                    .iter()
                    .all(|term| clause.contains(&normalize(term)));
                has_subject |= subject;
                has_modality |= modality;
                has_action |= action;
                has_scope |= scope;
                if subject && modality && action && scope {
                    return Ok(());
                }
            }
        }
        let missing = if !has_subject {
            "subject"
        } else if !has_modality {
            "modality"
        } else if !has_action {
            "action"
        } else if !has_scope {
            "scope"
        } else {
            "co-located semantics"
        };
        Err(ContractError {
            rule_id: rule.id,
            missing,
        })
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

fn push_block(blocks: &mut Vec<Block>, heading: &str, lines: &[&str]) {
    let text = normalize(&lines.join(" "));
    if !text.is_empty() {
        blocks.push(Block {
            heading: heading.to_owned(),
            text,
        });
    }
}

fn clauses(text: &str) -> impl Iterator<Item = &str> {
    text.split(['.', ';'])
        .filter(|clause| !clause.trim().is_empty())
}

fn parse_modality(clause: &str) -> Option<Modality> {
    if clause.contains("must not") {
        Some(Modality::Prohibited)
    } else if clause.contains("must") {
        Some(Modality::Required)
    } else if clause.contains("may") || clause.contains("can") {
        Some(Modality::Permitted)
    } else {
        None
    }
}

fn normalize(text: &str) -> String {
    text.to_ascii_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
