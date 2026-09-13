# Native review history recovery (retired)

Native review-history capture and recovery are no longer executable Codexy
paths. Their request schemas, provenance records, and CLI modes are rejected
before source reads, normalization, output creation, or output overwrite.

Existing raw pages, receipts, projections, and other historical artifacts MAY
remain immutable for audit or provenance. They MUST NOT be rewritten, treated as
current-head evidence, or used to establish readiness, completion, merge, or a
new review. Active review control uses the authenticated compact current-head
state described in [review profiles](review-profiles.md).
