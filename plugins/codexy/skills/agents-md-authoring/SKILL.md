---
name: agents-md-authoring
description: Use when creating, updating, reviewing, or relocating AGENTS.md instruction files, including repository root guidance, nested directory rules, instruction precedence, scope boundaries, and verification/readback expectations.
---

# AGENTS.md Authoring

## Purpose

Write AGENTS.md files as scoped operating instructions for agents working in a
directory tree. Keep them current, actionable, and easy to reconcile with
deeper instructions and higher-priority user or system directions.

## Workflow

1. Locate the target directory and read every governing `AGENTS.md` from the
   filesystem root or repository root down to that directory.
2. Identify the intended scope:
   - root guidance for repository-wide project structure and durable policies,
   - nested guidance only for stable local rules that should not apply
     elsewhere,
   - no new file when a short edit to an existing governing file is enough.
3. Check priority before writing:
   - system, developer, and direct user instructions outrank AGENTS.md,
   - deeper AGENTS.md files override higher ones inside their subtree,
   - AGENTS.md instructions apply only to files under the directory that
     contains them.
4. Draft instructions that are concrete and durable:
   - describe project purpose, structure, and where to look,
   - name hard requirements with `MUST` or `MUST NOT`,
   - prefer actionable guidance over generic agent advice,
   - avoid secrets, local-only paths, temporary notes, or duplicated workflow
     rules that already live in a canonical skill or doc.
5. Preserve ownership boundaries. Do not rewrite unrelated instructions,
   remove user-authored policy, or add broad repo rules while editing a narrow
   subtree file.
6. Verify and read back the final result:
   - inspect the edited file directly,
   - confirm the directory scope is correct,
   - check for conflicts with parent or child AGENTS.md files,
   - run the repository's documentation or formatting checks when available.

## Content Checklist

Include only sections that help agents act correctly:

- project or subtree purpose,
- scope and precedence notes,
- directory map or task routing table,
- coding, documentation, testing, or verification expectations,
- files or surfaces that must not be touched,
- handoff or evidence expectations when the repository requires them.

## Gates

- Do not add a nested AGENTS.md just to restate root guidance.
- Do not put credentials, private logs, machine-local state, or one-off task
  notes in AGENTS.md.
- Do not move executable workflow rules into AGENTS.md when the repository has
  a more specific skill, script, or policy file as the source of truth.
- Do not claim the instruction update is complete until the final file has been
  reread and its scope, precedence, and verification expectations are clear.

## Verification

For documentation-only AGENTS.md edits, run the repository's expected doc checks.
When no project-specific check exists, at minimum run:

```sh
git diff --check
test -f AGENTS.md
```

For nested files, replace `AGENTS.md` with the exact path and also list the
parent AGENTS.md files that were read.
