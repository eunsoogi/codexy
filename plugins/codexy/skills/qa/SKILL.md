---
name: qa
description: MUST use when verifying completed work, designing manual QA, checking real user surfaces, validating release candidates, acceptance criteria, repository settings, plugin behavior, or PR readiness.
---

# QA

## Purpose

QA turns claims into observable evidence. Automated tests are useful, but work
is not proven until the surface users, maintainers, or automation depend on has
been driven and inspected.

When replying in Korean, MUST follow [Natural Korean User Replies](../codex-orchestration/references/natural-korean-responses.md) for observable results while preserving exact QA evidence separately.

## Workflow

1. MUST list claims that need proof:
   - happy path,
   - riskiest edge,
   - regression path,
   - external surface named by the request.
2. Pick the faithful channel:
   - CLI: command, inputs, exit code, stdout/stderr marker.
   - HTTP/API: request, expected status, headers, body assertion.
   - Browser: URL, viewport, actions, visible text, screenshot or trace.
   - Desktop: app path, UI action, screenshot or accessibility evidence.
   - GitHub: PR, issue, review, branch, settings, or ruleset API state.
   - Completion handoff: final-answer or handoff artifact plus current
     `gh pr view` JSON through
     `scripts/validate-plugin-config --check-completion-handoff` when a
     completion claim could otherwise stop at an open PR; MUST include GraphQL
     `reviewThreads.nodes` when the artifact reports review feedback was
     addressed.
   - Plugin/config/docs: parser, schema, frontmatter, rendered preview, or
     structured dump.
   - Codexy architecture: `scripts/validate-plugin-config --check` when
     present, plus focused evidence for LSP config, MCP config, role metadata
     or custom agent TOMLs, and thread/worktree orchestration wording.
   - Code exploration: Codexy `codegraph` MCP output when the MCP is available,
     followed by direct file-read confirmation for edited files.
   - Child-owned PR review: owning child thread response, new head SHA, rerun
     verification, and parent-thread review-gate inspection.
3. MUST run automated checks first when available.
4. MUST drive the real surface for every user-visible or externally observable
   claim.
5. MUST record cleanup receipts for ports, sessions, temp directories, browser
   contexts, generated evidence, and worktrees.
6. MUST mark PASS only when the observable matches exactly enough to support the
   claim. Ambiguous evidence is inconclusive.

## Required Output

```text
Claim:
Channel:
Invocation:
Expected observable:
Evidence:
Result:
Cleanup:
```

## Gates

- MUST NOT call a scenario PASS without direct evidence.
- MUST NOT use a unit test as proof for a CLI, GitHub, browser, desktop, plugin,
  or marketplace behavior.
- MUST NOT ignore skipped checks; MUST list why they were skipped.
- MUST NOT leave QA-only servers, sessions, screenshots, traces, or temp files
  unaccounted for.
- MUST NOT pass Codexy plugin architecture QA without evidence for LSP, MCP,
  role metadata, custom agent TOMLs, thread, and worktree surfaces that changed.
- MUST NOT pass code-touching lane QA without Codexy `codegraph` MCP exploration
  evidence when the MCP is available, or an explicit unavailable-tool fallback.
- MUST NOT pass a child-owned lane when review feedback was fixed only in the
  parent thread. The owning child thread MUST validate the response or provide
  a documented non-change rationale.
- MUST NOT pass a completion handoff that claims done while a matching clean PR
  remains open unless the artifact states the explicit stop, wait, draft-only,
  no-merge, or leave-open instruction.

## Evidence Rules

- Screenshots prove visible state only for the captured viewport and time.
- GitHub API output proves repository state only for the returned PR, issue,
  branch, ruleset, or comment.
- Parser/schema checks prove syntax and shape, not semantic intent.
- `scripts/validate-plugin-config --check` proves the Codexy validator's
  configured contract for the current revision; pair it with direct file
  inspection for any newly added architecture claim.
- Child-thread review-response evidence proves only the lane and head it names;
  rerun parent PR review-gate checks before merge.
- If evidence was captured before a new commit, rerun it or label it stale.

## Failure Modes

- Saying "looks good" without evidence.
- Treating dry-run output as proof of a state-changing workflow.
- Testing the implementation path but not the user path.
- Forgetting cleanup after manual QA.
