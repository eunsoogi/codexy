# Feedback evidence

Use this contract when feedback informs an engineering decision or a proposed
harness improvement. It separates evidence from interpretation without deciding
process or outcome validity. The existing goal, success criteria, authority,
ownership, and
[adaptive work budget](../../orchestration/references/adaptive-work.md) remain
authoritative.

## Keep the claim and observation separate

For each feedback item used in a decision, MUST preserve:

- **Claim:** what the feedback asserts, including its applicable scope.
- **Observation:** what was actually seen, or explicitly that no observation is
  available. A critic's assertion MUST NOT become an observed fact by
  repetition.
- **Source:** the source type, retrievable evidence location when available, and
  original revision, time, environment, or conditions relevant to the claim.
- **Confidence:** high, medium, low, or unknown, with the evidence-based reason.
- **Limitations:** missing provenance, conflicts, uncertainty, dependence on
  other sources, and conditions under which the claim may not apply.

MUST distinguish these source types: actual execution, tool, external data,
human, LLM evaluator, self-evaluation, simulation, and heuristic. For a chain
such as an LLM summarizing a tool report, MUST retain both the immediate source
and the underlying origin when known. A simulation MUST NOT be represented as
actual execution in the target environment.

MUST keep unavailable provenance unknown; MUST NOT invent source details or
infer them from confident wording. A claim with missing evidence remains a
hypothesis. An accessible source location establishes traceability, not truth.

## Assess support and independence

MUST assign confidence to the claim in its stated scope, not automatically to
its source category. Use the following qualitative anchors; they are not scores
or automatic promotion thresholds.

| Confidence | Evidence basis                                                                                                           |
| ---------- | ------------------------------------------------------------------------------------------------------------------------ |
| High       | Relevant, traceable evidence directly supports the scoped claim, with material limitations and contradictions addressed. |
| Medium     | Relevant support exists, but coverage, repeatability, or independence has material limits.                               |
| Low        | Support is weak, indirect, unstable, or materially contradicted.                                                         |
| Unknown    | Available provenance or observations are insufficient to assess support.                                                 |

MUST distinguish independent corroboration from repetition. Multiple citations,
evaluators, or tool views derived from one observation share that origin and
MUST NOT count as independent evidence. If independence cannot be established,
MUST record it as unknown. Repeated observation may test stability under stated
conditions; it does not by itself broaden the claim's scope.

For example, three summaries of one failed run remain one underlying
observation. An independent reproduction under the relevant conditions may add
support; a passing run on a different revision does not erase the original
failure.

## Respond to weak or conflicting feedback

When feedback is weak, unstable, or conflicting, MUST choose a proportionate
response within the existing authority and finite budget:

- Seek an independent source or a direct observation that distinguishes the
  competing claims.
- Repeat a safe observation when it can resolve instability, preserving both the
  original and repeated results and their conditions.
- Backtrack to the last supported premise or decision when later steps depend on
  unsupported feedback.
- Lower confidence or retain unknown status when the conflict cannot be resolved
  with available evidence.

MUST explain which response was selected and what uncertainty remains. MUST NOT
average conflicting assertions into certainty, choose only convenient results,
or silently replace historical evidence with the latest result. Different
revisions or environments may explain disagreement; MUST check their relevance
before treating them as observations of the same claim.

Observation retries MUST NOT repeat unauthorized external side effects. Before
repeating an operation, MUST distinguish read-only observation from another
commitment, purchase, message, deployment, or destructive action. Prefer an
authorized status read or disposable reproduction when it answers the question.
When no safe authorized observation is available, MUST report the limit and
leave the dependent claim provisional.

MUST NOT permanently modify the harness on the basis of one low-confidence
feedback item. Additional evidence supports further evaluation; this contract
does not grant rule-promotion, global-memory, or mutation authority. MUST NOT
weaken the protected goal or success criteria to resolve a feedback conflict.

## Record only what the decision needs

Use existing task notes or an authorized evidence artifact. The optional
[feedback note](../templates/feedback-note.md) can keep a consequential item
compact. MUST NOT require a new file, global log, automated scoring service, or
permanent memory entry for every task. MUST preserve necessary provenance
without copying credentials, private raw logs, or unrelated personal data; use
an authorized reference or a minimal redacted observation instead.

This contract supplies evidence for later decisions. It MUST NOT substitute for
process/outcome assessment, verification of a critic, or the existing completion
and review gates.
