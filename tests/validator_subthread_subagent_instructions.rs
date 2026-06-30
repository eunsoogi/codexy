#[test]
fn codexy_workflows_distinguish_subthreads_from_subagents() -> Result<(), Box<dyn std::error::Error>>
{
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let orchestration =
        std::fs::read_to_string(root.join("plugins/codexy/skills/codex-orchestration/SKILL.md"))?;
    let thread_routing = std::fs::read_to_string(root.join(
        "plugins/codexy/skills/codex-orchestration/references/thread-and-worktree-routing.md",
    ))?;
    let classification =
        std::fs::read_to_string(root.join("plugins/codexy/skills/task-classification/SKILL.md"))?;
    let git_workflow =
        std::fs::read_to_string(root.join("plugins/codexy/skills/git-workflow/SKILL.md"))?;

    assert!(orchestration.contains("it is not a Codex subthread/worktree owner"));
    assert!(orchestration.contains("Subagents are not child-owned implementation owners"));
    assert!(thread_routing.contains("MUST NOT be treated as clean Codex"));
    assert!(classification.contains("MUST treat\n     them as different surfaces"));
    assert!(classification.contains("not child-owned Codex\n     subthread/worktree owners"));
    assert!(git_workflow.contains("Subagents are not child-owned implementation owners"));
    assert!(git_workflow.contains("commands MUST NOT be claimed as"));
    Ok(())
}
