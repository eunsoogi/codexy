# Legacy-rule equivalence matrix

This matrix records the six legacy routing surfaces consolidated by issue #547.
The write-once `baseline-v1` artifacts embedded in the runtime validator are the
authority for every trigger, semantic rule, output, and local reference; their
aggregate SHA-256 is pinned outside this manifest and its tests. Each destination
preserves its source workflow, required output, gates, evidence rules, and failure
modes. `git-workflow` and proof-driven completion are outside this engineering skill.

| Legacy route | Engineering destination | Baseline projection | Preserved invariant groups |
| --- | --- | --- | --- |
| `debugging` | [Diagnosis](diagnosis.md) | `baseline-v1/debugging.md` → [mapping](legacy-rule-mappings/debugging.json) | Reproduction before repair; evidence-led single-hypothesis experiments; minimal repair; original and regression proof; instrumentation cleanup. |
| `spec-driven-development` | [Specification](specification.md) | `baseline-v1/spec-driven-development.md` → [mapping](legacy-rule-mappings/spec-driven-development.json) | Governing-source and requirement extraction; atomic scope; happy, edge, regression, and external proofs; changed-file traceability. |
| `domain-driven-development` | [Domain modeling](domain-modeling.md) | `baseline-v1/domain-driven-development.md` → [mapping](legacy-rule-mappings/domain-driven-development.json) | Glossary; bounded-context ownership; explicit invariants and errors; boundary translation; domain and crossing-surface proof. |
| `test-driven-development` | [Test-driven development](test-driven-development.md) | `baseline-v1/test-driven-development.md` → [mapping](legacy-rule-mappings/test-driven-development.json) | Cheapest faithful proof; root-cause boundary; same RED/GREEN proof; regression and broader verification; no convenience-only performance repair. |
| `refactoring` | [Refactoring](refactoring.md) | `baseline-v1/refactoring.md` → [mapping](legacy-rule-mappings/refactoring.json) | Behavior-preserving boundaries; public-contract protection; structural 250-LOC compliance; focused verification; no unrelated cleanup. |
| `qa` | [Quality assurance](quality-assurance.md) | `baseline-v1/qa.md` → [mapping](legacy-rule-mappings/qa.json) | Claim inventory; faithful automated and real-surface checks; direct evidence; cleanup; no unsupported PASS. |

## Negative omission coverage

The production validator derives source-qualified identities and trigger clauses
from baseline-v1, then checks the strict six-route manifest projection, destination
equivalence, real entrypoint links, and legacy-bundle removal. Its mutation tests
MUST fail for baseline byte or inventory changes; omitted, extra, duplicate,
unknown, stale, or twice-mapped identities; duplicate or missing destinations;
modal, negation, lexical, trigger, broken-link, and outside-root-link changes.
