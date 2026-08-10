# Legacy-rule equivalence matrix

This matrix records the six legacy routing surfaces consolidated by issue #547.
Each destination preserves its source workflow, required output, gates, evidence
rules, and failure modes. `git-workflow` and proof-driven completion are outside
this engineering skill.

| Legacy route | Engineering destination | Preserved invariant groups |
| --- | --- | --- |
| `debugging` | [Diagnosis](diagnosis.md) | Reproduction before repair; evidence-led single-hypothesis experiments; minimal repair; original and regression proof; instrumentation cleanup. |
| `spec-driven-development` | [Specification](specification.md) | Governing-source and requirement extraction; atomic scope; happy, edge, regression, and external proofs; changed-file traceability. |
| `domain-driven-development` | [Domain modeling](domain-modeling.md) | Glossary; bounded-context ownership; explicit invariants and errors; boundary translation; domain and crossing-surface proof. |
| `test-driven-development` | [Test-driven development](test-driven-development.md) | Cheapest faithful proof; root-cause boundary; same RED/GREEN proof; regression and broader verification; no convenience-only performance repair. |
| `refactoring` | [Refactoring](refactoring.md) | Behavior-preserving boundaries; public-contract protection; structural 250-LOC compliance; focused verification; no unrelated cleanup. |
| `qa` | [Quality assurance](quality-assurance.md) | Claim inventory; faithful automated and real-surface checks; direct evidence; cleanup; no unsupported PASS. |

## Negative omission coverage

`engineering_skill_consolidation` removes every required section and preserved
invariant in turn. The contract test MUST fail for each omission, and it MUST
also fail while any of the six legacy skill bundles remains discoverable.
