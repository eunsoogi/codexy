//! #1424 source-contract regressions. These checks do not prove app permission,
//! task creation, concurrent execution, or integration conflict resolution.

use super::{LUNA, TERRA, check, fallback, hard_work_cases, request, selected};
use crate::support::TestResult;
use serde_json::json;

#[test]
fn routing_needs_no_extra_approval_or_path_allocation_fields() -> TestResult {
    // The resolver chooses recipients; actual authority, capacity and worktrees
    // belong to the host/supervisor, so no duplicate approval oracle is added.
    for pair in [LUNA, TERRA] {
        let classification = if pair == LUNA { "simple" } else { "general" };
        let mut creation = request(classification, pair);
        // Terra requires the same explicit hard-work evidence after main integration;
        // dispatch authority must not weaken the recipient selection contract.
        if pair == TERRA {
            creation["hard_work"] = hard_work_cases()?.remove(0);
        }
        check(creation.clone(), selected("generic", "create_thread", pair))?;
        let mut followup = creation.clone();
        followup["schema"] = json!("codexy.child-routing-request.v2");
        followup["codex_thread_operation"] = json!("send_message_to_thread");
        followup["codex_thread_direction"] = json!("parent_to_generic");
        followup["recipient_assignment"] = json!({"model":pair.0,"thinking":pair.1});
        check(
            followup,
            selected("parent_to_generic", "send_message_to_thread", pair),
        )?;
        // Scoped dispatch authority cannot manufacture unavailable capabilities.
        let mut unavailable = creation;
        unavailable["codex_thread_capabilities"] = json!(null);
        check(unavailable, fallback())?;
    }
    Ok(())
}

#[test]
fn canonical_guidance_preserves_scoped_authority_and_parallel_dispatch() -> TestResult {
    let root = codexy_runtime::paths::repository_root().join("plugins/codexy/skills");
    // Keep authority consumers linked to the canonical source while
    // retaining the host and supervisor boundaries checked below.
    for (path, required, obsolete) in [
        (
            "orchestration/references/parent-supervision.md",
            "without scanning paths or gating on path overlap",
            "",
        ),
        (
            "goal-lifecycle/SKILL.md",
            "MUST NOT be mistaken for host goal-tool permission",
            "",
        ),
    ] {
        let content = std::fs::read_to_string(root.join(path))?;
        let prose = content.split_whitespace().collect::<Vec<_>>().join(" ");
        assert!(
            prose.contains(required),
            "{path}: missing #1424 authority/dispatch contract"
        );
        assert!(
            obsolete.is_empty() || !prose.contains(obsolete),
            "{path}: stale dispatch gate"
        );
    }
    // Authority consumers link to the canonical decision; its meaning remains
    // a source-review obligation rather than another prose assertion here.
    for (consumer, target) in [
        (
            "orchestration/SKILL.md",
            "references/task-classification.md#orchestration-request-ownership",
        ),
        (
            "orchestration/references/child-routing-policy.md",
            "task-classification.md#orchestration-request-ownership",
        ),
        (
            "orchestration/references/thread-and-worktree-routing.md",
            "task-classification.md#orchestration-request-ownership",
        ),
    ] {
        assert_authority_link_resolves(&root, consumer, target)?;
    }
    let worktree = std::fs::read_to_string(
        root.join("orchestration/references/thread-and-worktree-routing.md"),
    )?;
    let prose = worktree.split_whitespace().collect::<Vec<_>>().join(" ");
    for invariant in [
        "Actual integration conflicts MUST be resolved before merge through the owning task's branch",
        "MUST NOT infer parent permission inheritance",
    ] {
        assert!(
            prose.contains(invariant),
            "missing preserved boundary: {invariant}"
        );
    }
    Ok(())
}

fn assert_authority_link_resolves(
    skills_root: &std::path::Path,
    consumer: &str,
    expected_target: &str,
) -> TestResult {
    let consumer_path = skills_root.join(consumer);
    let source = std::fs::read_to_string(&consumer_path)?;
    let target = source
        .split("](")
        .skip(1)
        .filter_map(|remainder| remainder.split_once(')').map(|(target, _)| target))
        .find(|target| *target == expected_target)
        .ok_or_else(|| format!("{consumer} does not link to the canonical authority section"))?;
    let (relative_path, fragment) = target
        .split_once('#')
        .ok_or_else(|| format!("{consumer} authority link has no heading fragment"))?;
    let target_document = consumer_path
        .parent()
        .ok_or_else(|| format!("{consumer} has no parent directory"))?
        .join(relative_path);
    let target_source = std::fs::read_to_string(&target_document)?;
    assert!(
        target_source
            .lines()
            .filter_map(plain_heading_anchor)
            .any(|anchor| anchor == fragment),
        "{consumer} authority link does not resolve to a heading in {}",
        target_document.display()
    );
    Ok(())
}

// The canonical section uses a plain ATX heading, so its fragment joins lower-case words.
fn plain_heading_anchor(line: &str) -> Option<String> {
    let heading = line.trim_start();
    let level = heading.bytes().take_while(|byte| *byte == b'#').count();
    if !(1..=6).contains(&level) {
        return None;
    }
    let title = heading
        .get(level..)?
        .strip_prefix(' ')?
        .trim()
        .trim_end_matches('#')
        .trim();
    if title.is_empty() {
        return None;
    }
    Some(
        title
            .split_whitespace()
            .map(|word| word.to_ascii_lowercase())
            .collect::<Vec<_>>()
            .join("-"),
    )
}
