---
name: qa
description: Use when verifying completed work, designing manual QA, checking real user surfaces, validating release candidates, acceptance criteria, repository settings, plugin behavior, or PR readiness.
---

# QA

## Purpose

QA turns claims into observable evidence. Automated tests are useful, but work
is not proven until the surface users, maintainers, or automation depend on has
been driven and inspected.

## Workflow

1. List claims that need proof:
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
   - Plugin/config/docs: parser, schema, frontmatter, rendered preview, or
     structured dump.
3. Run automated checks first when available.
4. Drive the real surface for every user-visible or externally observable
   claim.
5. Record cleanup receipts for ports, sessions, temp directories, browser
   contexts, generated evidence, and worktrees.
6. Mark PASS only when the observable matches exactly enough to support the
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

- Do not call a scenario PASS without direct evidence.
- Do not use a unit test as proof for a CLI, GitHub, browser, desktop, plugin,
  or marketplace behavior.
- Do not ignore skipped checks; list why they were skipped.
- Do not leave QA-only servers, sessions, screenshots, traces, or temp files
  unaccounted for.

## Evidence Rules

- Screenshots prove visible state only for the captured viewport and time.
- GitHub API output proves repository state only for the returned PR, issue,
  branch, ruleset, or comment.
- Parser/schema checks prove syntax and shape, not semantic intent.
- If evidence was captured before a new commit, rerun it or label it stale.

## Failure Modes

- Saying "looks good" without evidence.
- Treating dry-run output as proof of a state-changing workflow.
- Testing the implementation path but not the user path.
- Forgetting cleanup after manual QA.
