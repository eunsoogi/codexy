# Quality assurance

## Method

MUST turn each completion claim into observable evidence.

1. List the happy path, riskiest edge, regression path, and named external
   surface.
2. Choose the faithful channel: command and exit/output for CLI; request and
   response for API; visible state for browser or desktop; repository state for
   GitHub; parser, frontmatter, schema, structured dump, package validator, and
   installed invocation for plugin/configuration work.
3. For each claim, choose the cheapest faithful level that can detect its
   distinct failure. Do not automatically repeat the same observation at unit,
   integration, and end-to-end levels. Keep multiple levels only when they
   detect different failures or protect different boundaries.
4. Derive affected checks from changed executable boundaries, their callers,
   shared fixtures or dependencies, and the repository's existing suite and CI
   definitions. An unknown dependency or cross-cutting change expands the
   check set; it MUST NOT be silently skipped. A permanent impact graph is not
   required.
5. MUST run automated checks that cover the changed or claimed requirements
   first. MUST drive each user-visible or externally observable surface actually
   changed or claimed, using the channel required by that surface; a CLI
   behavior claim requires real CLI execution, while a documentation-only
   correction does not require installing unrelated components. A cheap local
   check MUST NOT replace required CLI, browser, desktop, package, CI,
   security, or platform evidence. MUST preserve authentic evidence and report
   cleanup and failures.
6. Distinguish a prose-only instruction edit from a behavior-changing
   instruction. Prose-only work uses structural readback and MUST NOT invent
   wording assertions or prose TDD. A behavior-changing instruction uses the
   existing bounded semantic-evaluation procedure only when its behavior or
   risk warrants it; it is not required for every documentation PR. A prose
   review or structural readback is not model-execution evidence; a later
   integrated evaluation remains a separate owned surface.
7. Bind PASS to the exact file state or head and account for temporary files,
   ports, sessions, screenshots, traces, and worktrees.

## Constraints

- MUST NOT infer real-surface readiness from a unit test, parser, dry run, or
  partial viewport.
- Code exploration uses available Codexy Codegraph followed by direct reads;
  language-aware changes use LSP or record its status.
- Installed architecture QA covers each changed MCP, LSP, role, agent, thread,
  worktree, or package surface rather than inferring it from configuration.
- Child-owned review fixes require the owning child response, new head, rerun
  proof, and refreshed review-thread state before readiness.
