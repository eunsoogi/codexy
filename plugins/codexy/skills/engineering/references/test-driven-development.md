# Test-driven development

## Method

For a v2 engineering boundary, MUST provide requirement-linked behavioral
verification whenever `engineering_tests_required` is true. MUST use faithful
RED/GREEN only when that boundary's `tdd_mode` is `required`. A legacy v1
`engineering_tdd_required` result preserves the old contract but MUST NOT be
read as a v2 test-first sequencing mandate.

1. Select one observable behavior and its root-cause boundary.
2. Choose the cheapest faithful unit, integration, CLI, API, browser, desktop,
   parser, schema, or command-output proof.
3. When `tdd_mode` is `required`, run RED before implementation and confirm it
   fails because behavior is missing or wrong, not because the harness is
   broken.
4. Make the smallest change and run the behavioral proof GREEN. An optional
   feature may use adequate existing or newly added behavioral coverage; an
   optional behavior-preserving refactor may use a GREEN or characterization
   baseline and MUST NOT manufacture RED.
5. Run broader verification sized to the affected boundary.

## Constraints

- Documentation, README, instruction-only skill prose, and reference Markdown
  MUST use direct structural readback, not manufactured RED, phrase mutations,
  or prose TDD.
- A reproducible defect requires faithful RED before the fix. When reproduction
  is unavailable, preserve that limitation, record the justified alternative,
  and still provide behavioral regression proof. Permission, destructive-state,
  and recovery risks require their invariants and failure expectations before
  implementation, but do not require RED for every helper.
- Mixed requests MUST apply these duties per boundary: behavioral tests remain
  required for engineering surfaces, while non-engineering surfaces use
  proportional proof.
- Replace mock-only assertions when they do not observe the requested behavior.
- Performance RED MUST measure the required workload once and record compile,
  execution, integration-target, and nested-process cost; skips, retries,
  sleeps, relaxed budgets, cache upgrades, or sharding alone MUST NOT satisfy
  it.
- Workflow and GitHub tests remain supporting evidence for their real surface.
