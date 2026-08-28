# Issue Intake

Before any Codexy-created issue mutation, the child MUST ask `$orchestration`
to apply its public **issue-intake receipt** contract and receive explicit
parent approval for the validated candidate.

The candidate evidence MUST come from a supported real producer or user-facing
surface and include:

- an all-state duplicate search and exact-versus-related classification;
- existing-owner exclusion and why a thin issue-sized change is necessary;
- a descriptive title and substantive problem, scope, acceptance, and
  verification content;
- the live repository label, milestone, and assignee taxonomies plus selected
  values; and
- the approving parent task identity and decision.

An exact duplicate, rejected or missing approval, unsupported synthetic or
same-class observation, ownable existing work, no-change outcome, invalid
metadata, or incomplete evidence MUST NOT create or mutate an issue. Preserve
it as a handoff-only result with the canonical existing issue when applicable.

Immediately before an approved mutation, MUST refresh duplicate and taxonomy
evidence. After mutation, MUST read back the issue number, URL, title, state,
labels, milestone, assignee, and body from GitHub. Connector or GitHub API
evidence is authoritative; a local receipt alone is not.
