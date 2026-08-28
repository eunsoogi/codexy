# Test-driven development

## Method

MUST use faithful RED/GREEN only for a boundary classified
`engineering_tdd_required` by orchestration.

1. Select one observable behavior and its root-cause boundary.
2. Choose the cheapest faithful unit, integration, CLI, API, browser, desktop,
   parser, schema, or command-output proof.
3. Run RED before implementation and confirm it fails because behavior is
   missing or wrong, not because the harness is broken.
4. Make the smallest change, run the identical proof GREEN, then refactor while
   keeping it green.
5. Run broader verification sized to the affected boundary.

## Constraints

- Documentation, README, instruction-only skill prose, and reference Markdown
  MUST use direct structural readback, not manufactured RED, phrase mutations,
  or prose TDD.
- Replace mock-only assertions when they do not observe the requested behavior.
- Performance RED MUST measure the required workload once and record compile,
  execution, integration-target, and nested-process cost; skips, retries, sleeps,
  relaxed budgets, cache upgrades, or sharding alone MUST NOT satisfy it.
- Workflow and GitHub tests remain supporting evidence for their real surface.
