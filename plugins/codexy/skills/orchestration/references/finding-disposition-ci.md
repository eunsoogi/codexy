# Authenticated Finding-Disposition CI (retired)

The authenticated finding-disposition capture and disposition path is no longer
executable Codexy behavior. Its locators, caller-supplied source/capture fields,
and disposition records are rejected before live GitHub reads, normalization,
or output mutation.

Existing source records MAY remain immutable for audit or provenance. They MUST
NOT waive a finding, establish current-head readiness, or replace the active
compact review-control state and its direct authenticated checks.
