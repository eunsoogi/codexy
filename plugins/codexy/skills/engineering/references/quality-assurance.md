# Quality assurance

## Purpose

QA turns claims into observable evidence. Automated tests are useful, but work
is not proven until the surface users, maintainers, or automation depend on has
been driven and inspected.

For every user-facing summary, MUST follow
[Plain-Language User Replies](../../orchestration/references/plain-language-user-replies.md)
while preserving exact QA evidence separately. When replying in Korean, MUST
also follow
[Natural Korean User Replies](../../orchestration/references/natural-korean-responses.md).

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
   - Completion handoff: final-answer or handoff artifact plus current external
     state when a completion claim could otherwise stop at an open PR; MUST
     include review-thread data when the artifact reports review feedback was
     addressed.
   - Plugin/config/docs: parser, schema, frontmatter, rendered preview, or
     structured dump.
   - Installed plugin architecture: the active project's package or schema
     validator when supplied, plus focused evidence for LSP config, MCP config,
     role metadata or custom-agent TOMLs, and task/thread/worktree behavior.
     A repository-only validator remains the active project's policy, not an
     installed Core prerequisite.
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
- MUST NOT pass installed plugin architecture QA without evidence for LSP, MCP,
  role metadata, custom agent TOMLs, thread, and worktree surfaces that changed.
- MUST NOT pass a code-touching lane QA without available `codegraph` MCP exploration
  evidence when the MCP is available, or an explicit unavailable-tool fallback.
- MUST NOT pass a child-owned lane when review feedback was fixed only in the
  parent thread. The owning child thread MUST validate the response or provide a
  documented non-change rationale.
- MUST NOT pass a completion handoff that claims done while a matching clean PR
  remains open unless the artifact states the explicit stop, wait, draft-only,
  no-merge, or leave-open instruction.

## Evidence Rules

- Screenshots prove visible state only for the captured viewport and time.
- GitHub API output proves repository state only for the returned PR, issue,
  branch, ruleset, or comment.
- Parser/schema checks prove syntax and shape, not semantic intent.
- A package or active-project validator proves only its configured contract for
  the current revision; pair it with direct file inspection for any newly added
  architecture claim.
- Child-thread review-response evidence proves only the lane and head it names;
  rerun parent PR review-gate checks before merge.
- If evidence was captured before a new commit, rerun it or label it stale.

## Failure Modes

- Saying "looks good" without evidence.
- Treating dry-run output as proof of a state-changing workflow.
- Testing the implementation path but not the user path.
- Forgetting cleanup after manual QA.
