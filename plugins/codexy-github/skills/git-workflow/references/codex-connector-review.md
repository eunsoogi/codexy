# Optional Manual Codex Connector Review

This procedure applies only when the parent has the connector and repository
policy requires one explicit manual `@codex review`. Otherwise it is not a
merge gate. Automatic connector review MUST remain disabled.

On one frozen exact head, the parent MUST:

1. finish affected local proof, required CI, and the owning lane's selected
   implementation review;
2. request exactly one manual connector review;
3. wait for its terminal result and batch all actionable findings into one
   child-owned repair; and
4. verify the repaired exact head, CI, reviews, comments, and thread resolution.

The owning child MUST NOT request connector review. It owns the repair and any
still-authorized selected-review delta check. If that review quota is exhausted,
it MUST repair every in-scope connector finding and return current-head proof
without fabricating approval or requesting another selected or connector
review.

Automatic, per-push, duplicate, unchanged-head, and piecemeal requests are
forbidden. Another connector review requires explicit maintainer authorization
after material scope expansion. Connector review does not replace human review,
CI, labels, title, issue linkage, thread resolution, merge authorization, or
merge-message validation.
