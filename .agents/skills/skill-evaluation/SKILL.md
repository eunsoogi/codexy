---
name: skill-evaluation
description: Use when evaluating a shipped skill with private cases for semantic behavior, authority boundaries, schema fidelity, or execution cost.
---

# Skill evaluation

## Purpose and boundary

This repository-only skill defines a small, manual evaluation procedure for a
deployed skill. It separates evaluator-owned cases and evidence from the prompts
that are shipped to users.

This skill MUST NOT build an execution service, an automatic LLM evaluator, a
score dashboard, or skill-specific answer files. It MUST NOT add evaluation
answers under `plugins/`. The procedure below is reusable; run-specific inputs,
results, measurements, and links MUST belong in the issue, PR, or a bounded
private artifact.

## Freeze and case ownership

1. The evaluator MUST freeze the exact skill revision before creating cases. The
   evaluator MUST record a run id, the issue or PR, frozen head SHA, skill path
   and revision, evaluator, and UTC start time.
2. The evaluator MUST create private cases after that freeze. Every case MUST
   have a stable hash. Skill authors MUST receive only the case hash and result
   summary; the evaluator MUST keep the input and expected behavior private.
3. If a case changes after creation, the evaluator MUST invalidate that case for
   the current run and MUST record the changed case as a separate run. The
   evaluator MUST NOT silently replace it.

## Required private case matrix

The evaluator MUST create at least two independent cases of each type:

- an existing-wording variant that tests the same boundary with different
  surface phrasing;
- a new domain that tests transfer without copying a known answer;
- quotation or negation that tests literal content versus the user's claim;
- insufficient input that tests uncertainty, refusal, or a request for what is
  missing.

Independence means that the cases do not share a copied answer or a single
surface cue. The evaluator MUST NOT turn the matrix into instruction-wording
tests.

## Per-case record format

The evaluator MUST keep one private record for every case. The values below are
placeholders, not holdout answers:

```yaml
run_id: <run identifier>
frozen_head: <exact commit SHA>
skill_revision: <skill path and revision>
case_hash: <stable case hash>
case_type: <existing-wording|new-domain|quotation-negation|insufficient-input>
input: <private input>
expected_behavior: <observable behavior and required fields>
prohibited_behavior: <semantic, causality, or authority violations>
evidence_alignment: <evidence that supports each material claim>
correction_retraction: <correction or retraction behavior, or not-applicable>
schema: <exact fields, types, nesting, and cardinality>
natural_language: <rubric observations>
result: <pass|fail|needs-review and failed dimensions>
input_tokens: <measured count or unavailable with reason>
output_tokens: <measured count or unavailable with reason>
execution_time_ms: <measured duration or unavailable with reason>
```

The evaluator MUST record the measurement source and the exact invocation
alongside the record. The evaluator MUST NOT estimate an unavailable cost value.

## Manual evaluation procedure

1. The evaluator MUST fix the head, create the private matrix, and hash each
   case.
2. The evaluator MUST invoke the deployed skill without changing its shipped
   instructions during the run. The evaluator MUST preserve the case hash and
   exact invocation in evaluator-owned records.
3. The evaluator MUST check machine output exactly. It MUST verify required
   field presence, field names, types, nesting, cardinality, and allowed values.
   A schema or presence mismatch MUST fail the case; the evaluator MUST NOT
   coerce it into prose.
4. The evaluator MUST judge natural-language content against the recorded
   behavior, not against a byte-for-byte answer. A paraphrase MAY pass when its
   meaning and evidence alignment are preserved.
5. The evaluator MUST apply the rubric: every material claim MUST be supported
   by available evidence; causal language MUST NOT exceed what the evidence
   establishes; and the response MUST respect authority boundaries, uncertainty,
   correction, and retraction. An uncorrected boundary violation MUST fail; an
   ambiguous result MUST be `needs-review`.
6. The evaluator MUST record input tokens, output tokens, and execution time for
   each case, then MUST publish only the case hash, status, failed dimensions,
   and cost summary to the issue or PR. The evaluator MUST NOT publish private
   inputs or expected answers.

When a comparable baseline exists, the evaluator MUST use the same private case
and a separately recorded frozen revision. The evaluator MUST report measured
improvement or semantic preservation. If there is no improvement or semantic
preservation fails, the evaluator MUST report the cause and the smallest
necessary follow-up; the evaluator MUST NOT lower the rubric or remove cases.

## Reuse in skill issues

When this procedure is copied into future skill-specific issues, the evaluator
MUST keep the same case matrix and record fields. The evaluator MUST attach only
case hashes, statuses, failed dimensions, and cost summaries. The evaluator MUST
keep all holdout answers and private inputs with the evaluator, outside the
deployed skill paths.
