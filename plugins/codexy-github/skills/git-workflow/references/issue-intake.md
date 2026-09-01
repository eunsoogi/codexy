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
5. Draft a substantive issue body with the required sections below.
6. Immediately before mutation, refresh the duplicate search and the live label,
   milestone, and assignee evidence.

Do not create an issue when the request is unauthorized or out of scope, an
exact duplicate exists, an existing owner covers the work, or the selected
metadata cannot be verified from the repository. Preserve the result as a
handoff with the canonical issue or pull request when one exists.

## Required issue body

The body MUST contain substantive content under each heading:

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
output alone is not proof of issue creation or metadata.
