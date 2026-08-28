---
name: agents-md-authoring
description:
  MUST use when creating, updating, reviewing, or relocating AGENTS.md instruction files, including repository root guidance, nested directory rules, instruction precedence, scope boundaries, and
  verification/readback expectations.
---

# AGENTS.md Authoring

## Workflow

1. MUST start discovery at the filesystem root and read every governing
   `AGENTS.md` through each ancestor directory down to the target before
   reviewing or writing instructions.
2. MUST choose the narrowest correct scope:
   - use root guidance for repository-wide structure and durable policy,
   - use nested guidance only for stable subtree rules that MUST NOT apply
     elsewhere,
   - MUST NOT create a nested file when an existing governing file can express
     the rule without changing its intended scope.
3. MUST apply instruction precedence:
   - system, developer, and direct user instructions outrank AGENTS.md,
   - deeper `AGENTS.md` files override parent files inside their subtree,
   - each `AGENTS.md` applies only to the directory tree rooted where it lives.
4. MUST edit only the governing file whose scope matches the requested rule:
   - mandatory agent instructions MUST use `MUST`,
   - prohibitions MUST use `MUST NOT`,
   - MUST preserve user-authored and unrelated instructions,
   - MUST NOT broaden or narrow a rule while relocating it.
5. MUST NOT add instructions that restate a parent file, expose secrets or
   machine-local state, or move executable procedures out of their canonical
   skill, script, or policy.
6. MUST reread the final target directly and verify that its directory scope,
   precedence, and relationship with governing parent or deeper files are
   correct. For review-only requests, report those findings without editing.
