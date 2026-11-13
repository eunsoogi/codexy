# Optional final review

Codexy has one optional final LLM reviewer: `codexy-sentinel`, using
`gpt-6.1-sol` / `high`. Inspector and the light, standard, and strict profile
selection are removed. MUST decide whether final review is useful from the
actual change and concrete uncertainty; no profile label mandates a reviewer.

When selected, Sentinel MUST review only after implementation and applicable
verification are ready. MUST NOT use Sentinel for intermediate progress checks
or repeatedly restart whole-change review while implementation is in flight. For
a child-owned lane, the owning child owns this final reviewer and repairs; the
parent consumes the evidence and retains merge/publication authority.

As an operating instruction, MUST limit Sentinel review to three assessments per
change, including the initial assessment and repair reassessments. This is
instruction-only guidance against review churn, not a hook, validator quota, or
mandatory review-count record. After the budget, MUST report remaining findings
instead of continuing a review loop or inventing a PASS. An actual unresolved
defect remains unresolved regardless of the review count.

A selected reviewer is not an automatic stack. Warden handles relevant security
boundaries and Auditor handles observable QA. Additional broad checks or a
connector review MUST have an explicit requirement or concrete unresolved risk;
loading the plugin alone MUST NOT add them.

Reviewer-state validity, exact-head binding, and results belong to the
[review lifecycle](review-lifecycle.md). Overall evidence and outcome judgment
belong to [proof-driven completion](../../proof-driven-completion/SKILL.md);
check selection and depth belong to
[quality assurance](../../engineering/references/quality-assurance.md).
