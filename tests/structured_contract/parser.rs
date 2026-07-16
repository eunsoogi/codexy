use super::Modality;

#[derive(Debug)]
pub(super) struct Block {
    headings: Vec<String>,
    text: String,
}

#[derive(Debug)]
pub(super) struct Clause {
    subject: String,
    tail: String,
    pub(super) modality: Modality,
    pub(super) conditional: bool,
    pub(super) inverted: bool,
}

pub(super) fn blocks(text: &str) -> Vec<Block> {
    let mut blocks = Vec::new();
    let mut headings: Vec<(usize, String)> = Vec::new();
    let mut lines = Vec::new();
    for line in text.lines() {
        if let Some((level, heading)) = atx_heading(line) {
            push_block(&mut blocks, &headings, &lines);
            while headings.last().is_some_and(|(prior, _)| *prior >= level) {
                headings.pop();
            }
            headings.push((level, canonical(heading)));
            lines.clear();
        } else {
            lines.push(line);
        }
    }
    push_block(&mut blocks, &headings, &lines);
    blocks
}

impl Block {
    pub(super) fn is_active(&self) -> bool {
        !self.headings.iter().any(|heading| {
            contains_phrase(heading, "historical") && !contains_phrase(heading, "non historical")
        })
    }

    pub(super) fn matches_heading(&self, expected: Option<&str>) -> bool {
        expected.is_none_or(|expected| {
            let expected = canonical(expected);
            self.headings.iter().any(|heading| heading == &expected)
        })
    }

    pub(super) fn clauses(&self) -> Vec<Clause> {
        let mut clauses = Vec::new();
        let mut inherited_subject = None;
        for sentence in self.text.split(['.', ';']) {
            for raw in split_repeated_modals(sentence) {
                let Some(mut clause) = Clause::parse(raw) else {
                    continue;
                };
                if clause.subject_is_implicit() {
                    if let Some(subject) = &inherited_subject {
                        clause.subject.clone_from(subject);
                    }
                } else {
                    inherited_subject = Some(clause.subject.clone());
                }
                clauses.push(clause);
            }
        }
        clauses
    }
}

impl Clause {
    fn parse(raw: &str) -> Option<Self> {
        let text = canonical(raw);
        let (subject, modality, tail) = split_modality(&text)?;
        let conditional = [
            "if available",
            "if possible",
            "unless",
            "only if",
            "provided that",
            "when possible",
            "where available",
        ]
        .iter()
        .any(|marker| contains_phrase(&text, marker));
        let inverted = modality == Modality::Prohibited
            && ["not", "never", "fail", "failed", "fails", "avoid", "avoids"]
                .iter()
                .any(|marker| contains_phrase(&tail, marker));
        Some(Self {
            subject,
            tail,
            modality,
            conditional,
            inverted,
        })
    }

    pub(super) fn subject_matches(&self, expected: &str) -> bool {
        let subject = self
            .subject
            .strip_prefix("the ")
            .or_else(|| self.subject.strip_prefix("a "))
            .or_else(|| self.subject.strip_prefix("an "))
            .unwrap_or(&self.subject);
        let expected = canonical(expected);
        subject == expected
            || subject
                .strip_prefix(&expected)
                .is_some_and(|tail| tail.starts_with(' '))
            || subject
                .strip_suffix(&expected)
                .is_some_and(|prefix| prefix.ends_with(' '))
    }

    pub(super) fn subject_is_implicit(&self) -> bool {
        self.subject.is_empty()
    }

    pub(super) fn tail_has(&self, terms: &[&str]) -> bool {
        terms.iter().all(|term| contains_phrase(&self.tail, term))
    }
}

fn atx_heading(line: &str) -> Option<(usize, &str)> {
    let trimmed = line.trim_start();
    let level = trimmed
        .chars()
        .take_while(|character| *character == '#')
        .count();
    if level == 0 || level > 6 || !trimmed[level..].starts_with(char::is_whitespace) {
        return None;
    }
    Some((level, trimmed[level..].trim().trim_end_matches('#').trim()))
}

fn push_block(blocks: &mut Vec<Block>, headings: &[(usize, String)], lines: &[&str]) {
    let text = lines.join(" ");
    if !text.trim().is_empty() {
        blocks.push(Block {
            headings: headings
                .iter()
                .map(|(_, heading)| heading.clone())
                .collect(),
            text,
        });
    }
}

fn split_modality(text: &str) -> Option<(String, Modality, String)> {
    let patterns = [
        ("must not", Modality::Prohibited),
        ("must never", Modality::Prohibited),
        ("is required not to", Modality::Prohibited),
        ("are required not to", Modality::Prohibited),
        ("must", Modality::Required),
        ("is required to", Modality::Required),
        ("are required to", Modality::Required),
        ("may", Modality::Permitted),
        ("can", Modality::Permitted),
        ("is allowed to", Modality::Permitted),
        ("are allowed to", Modality::Permitted),
    ];
    patterns.into_iter().find_map(|(pattern, modality)| {
        phrase_index(text, pattern).map(|index| {
            let tail = text[index + pattern.len()..].trim().to_owned();
            (text[..index].trim().to_owned(), modality, tail)
        })
    })
}

fn split_repeated_modals(sentence: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0;
    for (index, _) in sentence.match_indices(',') {
        let tail = sentence[index + 1..].trim_start();
        let tail = tail
            .strip_prefix("and ")
            .or_else(|| tail.strip_prefix("And "))
            .unwrap_or(tail);
        if starts_with_modality(tail) {
            parts.push(&sentence[start..index]);
            start = index + 1;
        }
    }
    parts.push(&sentence[start..]);
    parts
}

fn starts_with_modality(text: &str) -> bool {
    let text = canonical(text);
    [
        "must not",
        "must never",
        "is required not to",
        "are required not to",
        "must",
        "is required to",
        "are required to",
        "may",
        "can",
        "is allowed to",
        "are allowed to",
    ]
    .iter()
    .any(|pattern| text == *pattern || text.starts_with(&format!("{pattern} ")))
}

fn phrase_index(text: &str, phrase: &str) -> Option<usize> {
    text.match_indices(phrase).find_map(|(index, _)| {
        let before = text[..index].chars().next_back();
        let after = text[index + phrase.len()..].chars().next();
        (before.is_none_or(char::is_whitespace) && after.is_none_or(char::is_whitespace))
            .then_some(index)
    })
}

pub(super) fn contains_phrase(text: &str, phrase: &str) -> bool {
    phrase_index(&canonical(text), &canonical(phrase)).is_some()
}

pub(super) fn canonical(text: &str) -> String {
    text.to_ascii_lowercase()
        .split(|character: char| !character.is_ascii_alphanumeric() && character != '$')
        .filter(|token| !token.is_empty())
        .map(|token| match token {
            "keep" | "keeps" | "keeping" | "kept" => "retain",
            "throughout" | "while" | "during" => "during",
            _ => token,
        })
        .collect::<Vec<_>>()
        .join(" ")
}
