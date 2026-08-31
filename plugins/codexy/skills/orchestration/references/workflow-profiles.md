# Workflow profiles

Codexy uses three profiles: `light`, `standard`, and `strict`. Light is the
default for proportionate low-risk work. Standard covers non-trivial
single-owner work. Strict is required for high-risk, security, release,
multi-lane, and merge-sensitive work.

Strict work requires formal current-head proof and the applicable Sentinel
review. The invariant floor includes destructive-action safety, preservation of
unrelated changes, no force push, current-head readiness proof, and a maximum
of 250 physical lines for every governed file.

The executable profile contract is maintained by the packaged runtime
validator.
