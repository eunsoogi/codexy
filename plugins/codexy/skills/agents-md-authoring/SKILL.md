---
name: agents-md-authoring
description: Use when creating, updating, reviewing, or relocating AGENTS.md instruction files, including repository root guidance, nested directory rules, instruction precedence, scope boundaries, and verification/readback expectations.
---

# AGENTS.md Authoring

## Purpose

MUST write AGENTS.md files as scoped operating instructions for agents working in a
directory tree. MUST keep them current, actionable, and easy to reconcile with
deeper instructions and higher-priority user or system directions.

## Workflow

1. MUST locate the target directory and read every governing `AGENTS.md` from the
   filesystem root down through each ancestor directory to the target.
2. MUST identify the intended scope:
   - root guidance for repository-wide project structure and durable policies,
   - nested guidance only for stable local rules that MUST NOT apply
     elsewhere,
   - no new file when a short edit to an existing governing file is enough.
3. MUST check priority before writing:
   - system, developer, and direct user instructions outrank AGENTS.md,
   - deeper AGENTS.md files override higher ones inside their subtree,
   - AGENTS.md instructions apply only to files under the directory that
     contains them.
4. MUST draft instructions that are concrete and durable:
   - describe project purpose, structure, and where to look,
   - MUST name hard requirements with `MUST` or `MUST NOT`,
   - when creating or updating agent-facing instruction artifacts in this or
     another project, mandatory agent instructions MUST use `MUST` and
     prohibitions MUST use `MUST NOT`,
   - prefer actionable guidance over generic agent advice,
   - MUST NOT include secrets, local-only paths, temporary notes, or duplicated workflow
     rules that already live in a canonical skill or doc.
5. MUST preserve ownership boundaries. MUST NOT rewrite unrelated instructions,
   MUST NOT remove user-authored policy, and MUST NOT add broad repo rules while
   editing a narrow subtree file.
6. MUST verify and read back the final result:
   - MUST inspect the edited file directly,
   - MUST confirm the directory scope is correct,
   - MUST check for conflicts with parent or child AGENTS.md files,
   - MUST run the repository's documentation or formatting checks when available.

## Content Checklist

MUST include only sections that help agents act correctly:

- project or subtree purpose,
- scope and precedence notes,
- directory map or task routing table,
- coding, documentation, testing, or verification expectations,
- files or surfaces that MUST NOT be touched,
- handoff or evidence expectations when repository policy uses them.

## Gates

- MUST NOT add a nested AGENTS.md just to restate root guidance.
- MUST NOT put credentials, private logs, machine-local state, or one-off task
  notes in AGENTS.md.
- MUST NOT move executable workflow rules into AGENTS.md when the repository has
  a more specific skill, script, or policy file as the source of truth.
- MUST NOT claim the instruction update is complete until the final file has been
  reread and its scope, precedence, and verification expectations are clear.

## Verification

For documentation-only AGENTS.md edits, MUST run the repository's expected doc checks.
When no project-specific check exists, at minimum MUST run:

```sh
git diff --check
test -f AGENTS.md
```

For nested files, replace `AGENTS.md` with the exact path and also list the
parent AGENTS.md files that were read.
