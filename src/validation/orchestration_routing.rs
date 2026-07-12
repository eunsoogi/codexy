use std::{fs, path::Path};

use crate::paths::display_relative;
use crate::validation::orchestration_routing_semantics::{
    has_conflicting_luna_default, has_conflicting_sentinel_tier,
    has_conflicting_specialist_override, has_conflicting_tier_assignment,
};

const SKILL_PATH: &str = "skills/codex-orchestration/SKILL.md";

const REQUIRED_BULLETS: &[(&str, &[&str], &str)] = &[
    (
        "Root/orchestrator: MUST use `gpt-5.6-sol`",
        &[],
        "root/orchestrator must use gpt-5.6-sol",
    ),
    (
        "Generic implementation, debugging, integration, and QA child thread: MUST",
        &["model: \"gpt-5.6-terra\"", "reasoning_effort: \"high\""],
        "generic child thread must explicitly request gpt-5.6-terra/high",
    ),
    (
        "`gpt-5.6-luna` is only for repository discovery, cataloging, simple",
        &[
            "documentation drafting, bounded polling, and repetitive checks.",
            "MUST NOT use Luna as the blanket default for implementation, security review, or ambiguous reasoning.",
        ],
        "Luna must stay limited to enumerated low-risk mechanical work",
    ),
    (
        "Cost guidance: Luna is an optimization for bounded low-risk work, not a",
        &["quality-neutral replacement for Terra."],
        "Luna cost guidance must reject quality-neutral replacement claims",
    ),
    (
        "A named custom specialist TOML is the model and reasoning-effort source of",
        &["truth. MUST NOT pass model or reasoning-effort overrides."],
        "named custom specialists must keep their TOML model and reasoning effort",
    ),
    (
        "`codexy-sentinel` remains `gpt-5.6-sol` / `xhigh`.",
        &[
            "MUST NOT use Ultra.",
            "Custom-agent invocations MUST use `fork_turns=\"none\"` or a positive bounded count with a self-contained handoff.",
        ],
        "codexy-sentinel must remain gpt-5.6-sol/xhigh and MUST NOT use Ultra",
    ),
];

pub(super) fn check(plugin_root: &Path) -> Vec<String> {
    let path = plugin_root.join(SKILL_PATH);
    let Ok(skill) = fs::read_to_string(&path) else {
        return vec![format!(
            "{} could not be read for GPT-5.6 routing validation",
            display_relative(&path)
        )];
    };

    let Some(section) = routing_section(&skill) else {
        return vec![format!(
            "{} must define the GPT-5.6 routing matrix",
            display_relative(&path)
        )];
    };
    let bullets = policy_bullets(&section);
    let mut errors = REQUIRED_BULLETS
        .iter()
        .filter(|(start, clauses, _)| {
            bullets
                .iter()
                .find(|bullet| bullet.starts_with(start))
                .is_none_or(|bullet| clauses.iter().any(|clause| !bullet.contains(clause)))
        })
        .map(|(_, _, error)| format!("{} {error}", display_relative(&path)))
        .collect::<Vec<_>>();
    if bullets
        .iter()
        .any(|bullet| has_conflicting_specialist_override(bullet))
    {
        errors.push(format!(
            "{} named custom specialists must keep their TOML model and reasoning effort",
            display_relative(&path)
        ));
    }
    if bullets
        .iter()
        .any(|bullet| has_conflicting_tier_assignment(bullet))
    {
        errors.push(format!(
            "{} root/orchestrator must use gpt-5.6-sol; generic child thread must explicitly request gpt-5.6-terra/high",
            display_relative(&path)
        ));
    }
    if bullets
        .iter()
        .any(|bullet| has_conflicting_luna_default(bullet))
    {
        errors.push(format!(
            "{} Luna must remain limited to bounded mechanical work",
            display_relative(&path)
        ));
    }
    if bullets
        .iter()
        .any(|bullet| has_conflicting_sentinel_tier(bullet))
    {
        errors.push(format!(
            "{} codexy-sentinel must remain gpt-5.6-sol/xhigh",
            display_relative(&path)
        ));
    }
    errors
}

fn routing_section(skill: &str) -> Option<String> {
    let mut section = None;
    let mut fence: Option<Fence> = None;
    let mut in_comment = false;
    for line in skill.lines() {
        if line.starts_with("    ") || line.starts_with('\t') {
            continue;
        }
        let trimmed = line.trim_start();
        if let Some(marker) = fence {
            if marker.closes(trimmed) {
                fence = None;
            }
            continue;
        }
        if in_comment {
            if trimmed.contains("-->") {
                in_comment = false;
            }
            continue;
        }
        if trimmed.starts_with("<!--") {
            in_comment = !trimmed.contains("-->");
            continue;
        }
        if let Some(marker) = fence_marker(trimmed) {
            fence = Some(marker);
            continue;
        }
        if trimmed == "## GPT-5.6 Routing Matrix" {
            section = Some(String::new());
            continue;
        }
        if section.is_some() && trimmed.starts_with("## ") {
            break;
        }
        if let Some(section) = &mut section {
            section.push_str(line);
            section.push('\n');
        }
    }
    section
}

#[derive(Clone, Copy)]
struct Fence {
    marker: char,
    width: usize,
}

impl Fence {
    fn closes(self, line: &str) -> bool {
        let width = line.chars().take_while(|item| *item == self.marker).count();
        width >= self.width && line[width..].trim().is_empty()
    }
}

fn fence_marker(line: &str) -> Option<Fence> {
    let marker = line.chars().next()?;
    if !matches!(marker, '`' | '~') {
        return None;
    }
    let width = line.chars().take_while(|item| *item == marker).count();
    (width >= 3).then_some(Fence { marker, width })
}

fn policy_bullets(section: &str) -> Vec<String> {
    let mut bullets = Vec::new();
    for line in section.lines() {
        let trimmed = line.trim();
        if let Some(bullet) = trimmed.strip_prefix("- ") {
            bullets.push(bullet.to_owned());
        } else if !trimmed.is_empty() {
            if let Some(bullet) = bullets.last_mut() {
                bullet.push(' ');
                bullet.push_str(trimmed);
            }
        }
    }
    bullets
}
