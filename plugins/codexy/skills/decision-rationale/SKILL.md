---
name: decision-rationale
description: Use when a user has already chosen one option and asks to inspect its stated reason, evidence support, unsupported assumption, and reopen condition without changing the decision.
---

# Decision Rationale

Return one read-only rationale receipt only when the user has already chosen one
option and explicitly asks to inspect or explain that choice.

## Decline Boundary

MUST decline in plain prose, without receipt fields, when no chosen decision
exists or the request asks Codex to choose, recommend, approve, reject, cancel,
reopen, mutate state, fact-check evidence, review a current diff, prove
completion, change GitHub state, or route ownership.

## Receipt

For an eligible request, return exactly these four lines and nothing else:

```text
stated_reason: <supplied reason or unavailable>
evidence_support: <supplied evidence or unavailable>
unsupported_assumption: <one unsupported dependency or none>
reopen_condition: <one observable reconsideration condition or unavailable>
```

Serialize each field value reversibly on one line: replace each backslash with
`\\`, then each carriage return with `\r` and line feed with `\n`. Decode those
escapes to recover the supplied reason and evidence verbatim. Recording evidence
MUST NOT judge it correct or sufficient. An unsupported assumption is the
narrowest unproven dependency required by the stated reason, not invented
justification. The reopen condition is the first observable change that defeats
the reason or assumption; record it without reopening the decision. Use
`unavailable` when the supplied context cannot ground the field and `none` only
when no unsupported dependency is needed.

## Extraction Procedure

Treat every eligible request as a new analysis. MUST NOT look up, reuse, or
complete a canned answer for a familiar reason or evidence phrase. Extract the
four values in order from the request's supplied material:

1. Copy the user's stated reason into `stated_reason`. Preserve its meaning and
   wording rather than strengthening it. Use `unavailable` when no reason was
   supplied.
2. Identify only the explicit observations or claims the user supplied as
   evidence for that reason and record them in `evidence_support`. Do not add
   facts, validate the claims, or treat the option's outcome as evidence. Use
   `unavailable` when no explicit evidence was supplied.
3. Ask what single, smallest unproven dependency must hold for the stated reason
   to support the choice. Record that dependency in `unsupported_assumption`;
   use `none` only when the supplied reason and evidence require no additional
   unproven dependency. Do not invent a policy, consumer, threshold, or other
   context to fill this field.
4. Derive `reopen_condition` from the supplied material rather than inventing a
   hypothetical. Find the first observable change in that material that would
   break the assumption or the reason and make reconsideration relevant. If the
   material already supplies an observation, consequence, quote, or
   counterexample that grounds such a change, record that first grounded
   condition while preserving its attribution; do not return `unavailable` or
   replace it with a future condition. Use `unavailable` only when no supplied
   material grounds any observable condition. Record it without reopening,
   judging, or changing the decision.

Keep the distinction between the user's evidence and the analysis of what it
does or does not establish. When wording is quoted or negated, treat the quote
or negation as supplied content; do not silently convert it into the user's own
claim. When the request is incomplete or ambiguous, preserve the available
material and use `unavailable` where the corresponding value cannot be grounded
instead of guessing.

MUST NOT add a preface, explanation, status, verdict, recommendation, second
receipt, or additional field.
