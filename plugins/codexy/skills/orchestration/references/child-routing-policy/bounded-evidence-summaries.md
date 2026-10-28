# Bounded evidence summaries

An ordinary Worker MAY use `classification="bounded_evidence"` for a read-only
summary of explicitly supplied documents when no named specialist role matches.
The assignment MUST name 1–3 documents totalling at most 12,000 Unicode
characters and require 1–20 concise output items (at most 40 words each). It
MUST prohibit writes, command execution, external research, implementation,
recommendations, and decisions. Input scope and access authority MUST already be
established. Unknown authority or missing inputs MUST remain fail-closed. The
caller MUST count the complete supplied inputs, not selected excerpts from
larger material that the Worker would need to explore. Work exceeding these
bounds MUST use the general classification or an applicable named specialist.
General work still defaults to Luna/Max unless a hard-work criterion is
evidenced.

For example, summarizing two supplied release notes into ten factual bullets, or
restating three supplied product descriptions in a comparison table, qualifies.
Wording is human-judged, so `deterministic_oracle` may be false. Investigating
source dependencies, reviewing correctness, resolving conflicting requirements,
or implementing a multi-boundary change does not qualify.

The resolver retains the compatibility name `simple_predicates` for this class.
`fixed_scope`, `low_risk_reversible`, and `no_unresolved_decision` MUST all be
true; they certify the scope, safety, and settled decisions above.
`deterministic_oracle` MUST still be supplied but may be false. The optional
`bounded_evidence` object MUST contain exactly these required facts:

```json
{
  "read_only": true,
  "supplied_inputs_only": true,
  "input_count": 2,
  "input_char_count": 8000,
  "output_item_limit": 10
}
```

Both booleans MUST be true. Counts MUST be integers in the limits above. Absent
bounds or out-of-range values MUST select the safe fallback; malformed or
unknown fields MUST be rejected. The old `simple` class still requires all four
original predicates. A general request MUST be evaluated by its own safety and
hard-work facts; bounded facts MUST NOT override an evidenced hard-work
selection. A capability mismatch MUST NOT substitute the legacy Sol/Medium
delivery route, a default model, or a specialist whose role does not match. Creation MUST send the
selected Luna/Max pair explicitly; recipient-bound follow-ups MUST preserve the
assigned pair through the existing v2 route. Requested settings and authentic
task-creation readback MUST remain distinct from effective host settings, which
MUST be reported unavailable when unexposed.
