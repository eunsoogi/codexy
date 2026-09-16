# Issue creation

Use this guide before creating a GitHub issue in any repository. It is
repository-generic and does not depend on repository-specific files or policy.

## Before creating an issue

1. Confirm that issue creation is authorized and within the requested repository
   and scope.
2. Search both open and closed issues using the problem, affected surface, and
   likely terminology. Classify exact matches separately from related work.
3. Check whether an existing issue or pull request already owns the work. Link
   to that owner instead of creating a competing issue.
4. Read the live repository taxonomies for labels, milestones, and assignees.
   Select only values that exist in those taxonomies.
5. If the repository or maintainer selected the issue template below, draft a
   substantive body with those sections; otherwise follow the requested or
   repository-owned format.
6. Immediately before mutation, refresh the duplicate search and the live label,
   milestone, and assignee evidence.

Do not create an issue when the request is unauthorized or out of scope, an
exact duplicate exists, an existing owner covers the work, or the selected
metadata cannot be verified from the repository. Preserve the result as a
handoff with the canonical issue or pull request when one exists.

## Transfer an authorized plan into an issue

A plan is supporting input and MUST NOT grant issue-creation authority. MUST use
this section only after the existing authorization, duplicate, taxonomy,
assignee, milestone, and owner checks are satisfied. When an authorized issue is
derived from a plan, its body MUST be understandable without the plan file or
prior conversation and MUST include each of these facts:

```markdown
## Background

<problem, affected behavior, and current evidence>

## Objectives

<observable outcomes and the artifact or contract this issue produces>

## Scope and exclusions

<concrete paths or behavior in scope, followed by explicit non-goals>

## Prerequisite artifacts

- <actual issue number or artifact> — <the required output or contract, not only
  an ID>

## Completion criteria

<observable conditions that show this issue is complete>

## Verification

<exact checks, readbacks, or authentic surfaces that prove the criteria>

## Owned paths

<exact files or directories this issue may change>

## Stop/report conditions

<failure or decision boundary and the owner of the next decision>
```

MUST preserve plan exclusions, dependency order, and read-only boundaries in the
issue body. MUST NOT publish only a `.plans/<topic>.md` path or a conversation
link in place of the background, objectives, scope, prerequisites, completion,
verification, ownership, or stop conditions. A plan's completion or ownership
claim MUST NOT replace live Git, GitHub, goal, review, or verification evidence.

When a plan contains draft task or dependency IDs, MUST convert each to the
actual GitHub issue number returned by authorized creation before using it as a
reference. After each registration, MUST read back the actual issue body and
metadata from GitHub; a draft ID or local plan reference is not registration
proof. MUST NOT create or register issues merely because a plan was requested.

## Issue title

The installed component retains this existing issue-title check on supported
issue creation and title-edit paths. The title MUST be written in English using
descriptive, sentence-style wording and begin with an ASCII uppercase letter. It
MUST state the problem or requested change in plain prose and MUST NOT begin
with a category, type, scope, bracket, colon, or dash. For example, use
`Reduce
CI build time`, not a category label such as `CI: reduce CI build time`
or `[CI]
Reduce CI build time`. This is a narrow title contract, not a general
mutation allowlist or body gate. A repository or maintainer may choose
additional repository-owned conventions; installing this plugin alone does not
create those additional rules. A syntax check does not replace human review for
meaning.

## Optional issue body template

When this repository or maintainer selects this template, the body MUST contain
substantive content under each heading. Without that selection, no heading or
footer is required by this plugin; follow the user or repository-owned format.

## Problem

Describe the observed problem, affected behavior, and evidence.

## Scope

State what is included, what is excluded, and the affected repository surface.

## Acceptance Criteria

State the observable conditions that will show the issue is resolved.

## Verification

State the tests, checks, or readbacks that will prove the acceptance criteria.

## After creation

Use an authenticated GitHub connector or API readback after mutation. Confirm
the issue number, URL, title, state, labels, milestone, assignee, and body from
GitHub. The authenticated readback is authoritative; a local request or local
output alone is not proof of issue creation or metadata. For a plan-derived
issue, also confirm that the read-back body contains the self-contained plan
facts and actual issue-number references required above.
