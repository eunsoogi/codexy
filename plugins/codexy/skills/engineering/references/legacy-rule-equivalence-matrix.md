# Legacy-rule equivalence matrix

This matrix records the six legacy routing surfaces consolidated by issue #547.
Each destination preserves its source workflow, required output, gates, evidence
rules, and failure modes. `git-workflow` and proof-driven completion are outside
this engineering skill.

| Legacy route | Engineering destination | Normalized rule inventory | Preserved invariant groups |
| --- | --- | --- | --- |
| `debugging` | [Diagnosis](diagnosis.md) | 18 mandatory/prohibited rules; 10 output fields; 2 shared contracts | Reproduction before repair; evidence-led single-hypothesis experiments; minimal repair; original and regression proof; instrumentation cleanup. |
| `spec-driven-development` | [Specification](specification.md) | 13 mandatory/prohibited rules; 7 output fields | Governing-source and requirement extraction; atomic scope; happy, edge, regression, and external proofs; changed-file traceability. |
| `domain-driven-development` | [Domain modeling](domain-modeling.md) | 16 mandatory/prohibited rules; 7 output fields | Glossary; bounded-context ownership; explicit invariants and errors; boundary translation; domain and crossing-surface proof. |
| `test-driven-development` | [Test-driven development](test-driven-development.md) | 18 mandatory/prohibited rules; 11 output fields | Cheapest faithful proof; root-cause boundary; same RED/GREEN proof; regression and broader verification; no convenience-only performance repair. |
| `refactoring` | [Refactoring](refactoring.md) | 36 mandatory/prohibited rules; 9 output fields | Behavior-preserving boundaries; public-contract protection; structural 250-LOC compliance; focused verification; no unrelated cleanup. |
| `qa` | [Quality assurance](quality-assurance.md) | 16 mandatory/prohibited rules; 7 output fields; 2 shared contracts | Claim inventory; faithful automated and real-surface checks; direct evidence; cleanup; no unsupported PASS. |

## Negative omission coverage

`engineering_skill_consolidation` validates the six normalized source records,
their destination references, all mandatory/prohibited rules, required outputs,
shared contracts, and the engineering frontmatter route. It MUST fail for every
rule omission and for missing, duplicate, unknown, or stale inventory entries.
