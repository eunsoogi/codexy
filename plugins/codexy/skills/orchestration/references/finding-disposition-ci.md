# Authenticated Finding-Disposition CI

The `authenticated_finding_disposition` source MUST combine four authenticated
GitHub reads for the exact pull-request head:

- `gh pr view` MUST provide the fixed `statusCheckRollup` projection.
- Branch protection MUST provide the configured required-check state.
- Paginated check-run and check-suite reads MUST provide the registered
  exact-head inventories.

The source MUST also include a fixed GraphQL pull-request comment lookup bound
to the exact repository, owning issue, PR, base, head, finding ID/path,
immutable unminimized OWNER/MEMBER comment, narrow non-waiver body, and accepted
model tuple. The rollup and registered inventories MUST be non-empty and
terminal-success, and configured required contexts and app IDs MUST match the
observed runs.

The transcription preamble MUST use the fixed title, attribution, and a
model-routing subject with a bounded lexical descriptor; free-form or operative
pre-heading text is not authoritative and MUST be rejected.

An authenticated `known_empty` required-check configuration is distinct from an
unavailable or incomplete source. It MUST NOT be treated as an inventory of
future jobs. The source may claim only `registered_check_runs` and registered
check-suite coverage; it MUST NOT invent a universal expected-job list.

The producer MUST derive finding IDs, paths, and kinds from the authenticated
prior delta, reject caller-supplied source, capture, classification, or IDs,
and reread both source families at producer, build, and completion-handoff.
