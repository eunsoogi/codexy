use std::{fs, path::Path};

use crate::paths::display_relative;
use crate::validation::orchestration_routing_semantics::{
    has_conflicting_luna_default, has_conflicting_sentinel_tier,
    has_conflicting_specialist_override, has_conflicting_tier_assignment,
};

mod assignments;
mod evidence;
mod policy;
mod required_bullets;
mod rules;

#[cfg(test)]
mod tests;

use policy::{affirmative_field_values, policy_instructions, sections_for_heading};
use required_bullets::missing_required_bullets;
use rules::{
    ACTIVE_TIER_STARTS, DELIVERY_POLICY, RECIPIENT_ROUTING_BULLETS, ROUTING_REQUIRED_BULLETS,
    simple_route_is_affirmative,
};

const SKILL_PATH: &str = "skills/orchestration/SKILL.md";
const RECIPIENT_ROUTING_HEADING: &str = "## Recipient Model Routing";

pub(super) fn check(plugin_root: &Path) -> Vec<String> {
    let path = plugin_root.join(SKILL_PATH);
    let Ok(skill) = fs::read_to_string(&path) else {
        return vec![format!(
            "{} could not be read for GPT-5.6 routing validation",
            display_relative(&path)
        )];
    };
    check_skill(&path, &skill)
}

pub(super) fn check_skill(path: &Path, skill: &str) -> Vec<String> {
    let routing_sections = sections_for_heading(skill, "## GPT-5.6 Routing Matrix");
    if routing_sections.is_empty() {
        return vec![format!(
            "{} must define the GPT-5.6 routing matrix",
            display_relative(&path)
        )];
    }
    let routing_starts = ROUTING_REQUIRED_BULLETS
        .iter()
        .map(|(start, _, _)| *start)
        .chain(ACTIVE_TIER_STARTS.iter().copied())
        .collect::<Vec<_>>();
    let routing_bullets = routing_sections
        .iter()
        .map(|section| policy_instructions(section, &routing_starts))
        .collect::<Vec<_>>();
    let mut errors = routing_bullets
        .iter()
        .flat_map(|bullets| missing_required_bullets(&path, bullets, ROUTING_REQUIRED_BULLETS))
        .collect::<Vec<_>>();
    if routing_bullets.iter().flatten().any(|bullet| {
        bullet.starts_with("Candidate simple work") && !simple_route_is_affirmative(bullet)
    }) {
        errors.push(format!(
            "{} simple-work Luna/max candidates must require every bounded-work predicate",
            display_relative(&path)
        ));
    }
    let recipient_sections = sections_for_heading(skill, RECIPIENT_ROUTING_HEADING);
    if recipient_sections.is_empty() {
        errors.push(format!(
            "{} must define recipient model routing policy",
            display_relative(&path)
        ));
        return errors;
    }
    let recipient_starts = RECIPIENT_ROUTING_BULLETS
        .iter()
        .map(|(start, _, _)| *start)
        .chain(ACTIVE_TIER_STARTS.iter().copied())
        .chain(assignments::INSTRUCTION_STARTS.iter().copied())
        .chain(evidence::ROUTES.iter().map(|(marker, ..)| *marker))
        .collect::<Vec<_>>();
    let recipient_bullets = recipient_sections
        .iter()
        .flat_map(|section| policy_instructions(section, &recipient_starts))
        .collect::<Vec<_>>();
    errors.extend(missing_required_bullets(
        &path,
        &recipient_bullets,
        RECIPIENT_ROUTING_BULLETS,
    ));
    let delivery_assignments = assignments::delivery(&recipient_bullets, &recipient_starts);
    for (direction, model, effort, error) in [
        (
            "parent-to-generic-child delivery must pass",
            "gpt-5.6-terra",
            "high",
            "parent-to-generic-child messages must use recipient gpt-5.6-terra/high",
        ),
        (
            "child-to-root delivery must pass",
            "gpt-5.6-sol",
            "medium",
            "child-to-root messages must use recipient gpt-5.6-sol/medium",
        ),
    ] {
        if delivery_assignments.iter().any(|(found, assignment)| {
            let models = affirmative_field_values(assignment, "model");
            let efforts = affirmative_field_values(assignment, "thinking");
            *found == direction
                && (!models.contains(&model)
                    || models.iter().any(|value| *value != model)
                    || !efforts.contains(&effort)
                    || efforts.iter().any(|value| *value != effort))
        }) {
            errors.push(format!("{} {error}", display_relative(&path)));
        }
        if assignments::has_negated(&recipient_bullets, &recipient_starts, direction) {
            errors.push(format!("{} {error}", display_relative(&path)));
        }
    }
    for (conflict, message) in [
        (
            has_conflicting_specialist_override as fn(&str) -> bool,
            "named custom specialists must keep their TOML model and reasoning effort",
        ),
        (
            has_conflicting_tier_assignment,
            "root/orchestrator must use gpt-5.6-sol; generic child thread must explicitly request gpt-5.6-terra/high",
        ),
        (
            has_conflicting_luna_default,
            "Luna must remain limited to bounded mechanical work",
        ),
        (
            has_conflicting_sentinel_tier,
            "codexy-sentinel must remain gpt-5.6-sol/xhigh",
        ),
    ] {
        if routing_bullets
            .iter()
            .flatten()
            .map(String::as_str)
            .chain(recipient_bullets.iter().filter_map(|bullet| {
                if bullet.starts_with("Captured #433 parent-to-generic-child evidence")
                    || bullet.starts_with("Reverse child-to-root evidence")
                {
                    None
                } else {
                    bullet
                        .strip_prefix(DELIVERY_POLICY)
                        .filter(|suffix| !suffix.trim().is_empty())
                        .or((!bullet.starts_with(DELIVERY_POLICY)).then_some(bullet))
                }
            }))
            .any(|bullet| conflict(bullet))
        {
            errors.push(format!("{} {message}", display_relative(&path)));
        }
    }
    for (marker, recipient, sender, thread, effort, direction) in evidence::ROUTES {
        if evidence::invalid(
            &recipient_bullets,
            &recipient_starts,
            marker,
            recipient,
            sender,
            thread,
            effort,
        ) {
            errors.push(format!(
                "{} {direction} evidence must pass recipient {recipient}/{effort}",
                display_relative(&path)
            ));
        }
    }
    if !errors.is_empty() {
        errors.push(format!(
            "{} has an unreviewed, moved, or changed normative rule",
            display_relative(&path)
        ));
    }
    errors
}
