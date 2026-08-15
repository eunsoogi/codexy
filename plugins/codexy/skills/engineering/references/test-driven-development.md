# Test-driven development

## Purpose

Make the desired engineering behavior fail for the right reason before changing
production code. Then make the smallest change that turns the proof green and
MUST keep broader verification proportional to risk.

Documentation, README, instruction-only skill prose, and reference Markdown MUST
NOT use manufactured RED tests, phrase mutations, or prose TDD. MUST verify
those edits by direct diff and readback plus applicable existence, link, render,
frontmatter, or package-structure checks.

## RED-GREEN-REFACTOR Loop

1. MUST select one behavior from the active spec or issue.
2. MUST choose the cheapest faithful proof:
   - unit test for pure logic,
   - integration test for wiring, adapters, persistence, or process boundaries,
   - CLI/API/browser/desktop scenario for user-facing behavior,
   - parser, schema, or command-output check for structured config, plugin
     metadata, or workflow rules.
3. MUST run the proof before implementation and capture RED.
4. MUST confirm RED fails because the behavior is missing or wrong, not because
   the harness is broken.
5. Implement the smallest change that satisfies the proof.
6. MUST run the same proof and capture GREEN.
7. Refactor only after GREEN, keeping proofs green after each cleanup.
8. MUST run broader checks sized to blast radius before PR, handoff, or merge.

## Root-Cause And Harness Discipline

- MUST identify the root-cause boundary before selecting a repair RED.
- MUST place permutation cases at the pure or unit layer when observable
  behavior does not require filesystem, process, network, or UI wiring.
- MUST keep one faithful boundary test when observable CLI, process, discovery,
  persistence, network, or UI behavior requires that boundary.
- A new standalone integration crate MUST document required isolation.
  Otherwise, MUST add the case to an existing domain integration target.
- Performance RED MUST measure the original required workload exactly once.
- Performance RED evidence MUST record compile cost, execution cost,
  integration-target count, and nested subprocess or build count.
- MUST NOT satisfy performance acceptance with skips, filters, retries, sleeps,
  relaxed budgets, cache or runner upgrades as the sole fix, sharding alone, or
  a representative subset.

## Required Output

```text
Behavior:
Root-cause boundary:
Harness cost:
Integration target:
Performance RED:
RED command:
RED reason:
GREEN command:
Broader verification:
Refactor notes:
Not covered:
```

## Gates

- If the proof passes before implementation, rewrite the proof.
- If RED is caused by syntax, setup, or test harness failure, fix the proof
  before production edits.
- If the proof only checks a mock call, replace or supplement it with an
  observable behavior assertion.
- If broader verification fails, debug before claiming completion.

## Evidence Rules

- RED and GREEN MUST be the same proof unless there is a documented reason to
  change it.
- The proof MUST be faithful to the requested behavior, not merely convenient.
- For plugin skills and reference Markdown, MUST use structural readback rather
  than executable tests of wording.
- For workflow or GitHub behavior, a local test is supporting evidence; the
  matching GitHub or CLI surface MUST still be inspected.

## Failure Modes

- Writing implementation first and inventing a test afterward.
- Accepting a RED caused by a typo or broken dependency.
- Over-mocking the system so the regression would still pass.
- Treating a narrow unit test as proof for a broad user-visible workflow.
