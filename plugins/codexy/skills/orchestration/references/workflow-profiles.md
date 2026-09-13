# Workflow profiles

Codexy uses three profiles: `light`, `standard`, and `strict`. Light is the
default for proportionate low-risk work. Standard covers non-trivial
single-owner work. Strict is required for high-risk, security, release,
explicit audit, materially shared executable-contract changes, and
merge-sensitive work. Delegation and lane count alone do not select the strict
profile.

Strict work requires formal current-head proof and the applicable Sentinel
review. The invariant floor includes destructive-action safety, preservation of
unrelated changes, no force push, current-head readiness proof, and a maximum of
250 physical lines for every governed file.

The GitHub surface does not by itself select review guidance. The route table
selects it for review workflows and explicitly chosen review procedures.

An unavailable optional source diagnostic is not an unsupported primary
operation. When the primary operation is otherwise authorized, the agent MUST
continue it and MUST preserve the diagnostic as `unsupported` or `unknown`, and
MUST NOT turn it into `PASS` or a permission barrier. Fail-closed routing for an
unsupported primary request or missing required contract remains unchanged.

The executable profile contract is maintained by the packaged runtime validator.
