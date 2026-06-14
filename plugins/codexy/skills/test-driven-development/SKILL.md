---
name: test-driven-development
description: Use when implementing a feature, bug fix, behavior change, refactor, validator, harness, CLI behavior, documentation rule, plugin skill, workflow rule, or release automation before production edits.
---

# Test-Driven Development

## Purpose

Make the desired behavior fail for the right reason before changing production
code or durable content. Then make the smallest change that turns the proof
green and keep broader verification proportional to risk.

## RED-GREEN-REFACTOR Loop

1. Select one behavior from the active spec or issue.
2. Choose the cheapest faithful proof:
   - unit test for pure logic,
   - integration test for wiring, adapters, persistence, or process boundaries,
   - CLI/API/browser/desktop scenario for user-facing behavior,
   - parser, schema, frontmatter, rendered-output, or command-output check for
     docs, config, plugin metadata, or workflow rules.
3. Run the proof before implementation and capture RED.
4. Confirm RED fails because the behavior is missing or wrong, not because the
   harness is broken.
5. Implement the smallest change that satisfies the proof.
6. Run the same proof and capture GREEN.
7. Refactor only after GREEN, keeping proofs green after each cleanup.
8. Run broader checks sized to blast radius before PR, handoff, or merge.

## Required Output

```text
Behavior:
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

- RED and GREEN should be the same proof unless there is a documented reason to
  change it.
- The proof must be faithful to the requested behavior, not merely convenient.
- For plugin skills, content validation can be the test only if it checks the
  required behavior: frontmatter, metadata, required sections, routing terms,
  and old-path removal when moving files.
- For workflow or GitHub behavior, a local test is supporting evidence; the
  matching GitHub or CLI surface must still be inspected.

## Failure Modes

- Writing implementation first and inventing a test afterward.
- Accepting a RED caused by a typo or broken dependency.
- Over-mocking the system so the regression would still pass.
- Treating a narrow unit test as proof for a broad user-visible workflow.
