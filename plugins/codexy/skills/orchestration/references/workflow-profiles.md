# Risk-based verification

Codexy does not classify work into light, standard, or strict profiles. MUST
choose checks from the changed behavior, acceptance criteria, and concrete
material risks, not a profile label, delegation count, or number of lanes.

Security, permission, destructive-action, release, and shared-contract risks
still require their applicable checks and honest evidence. MUST preserve
unrelated changes, avoid force push, bind readiness claims to the current head,
and keep every governed file within 250 physical lines. Removing profile tiers
MUST NOT waive these safety and evidence obligations.

LLM final review is optional. MUST use [final review](review-profiles.md) when
choosing whether Sentinel adds useful independent scrutiny. Warden's targeted
security review and Auditor's observable QA are separate responsibilities; they
MUST NOT become an automatic reviewer stack.

The GitHub surface does not by itself select review guidance. Legacy profile
records remain historical evidence, not a new review decision or a current-head
PASS. MUST NOT silently relabel a historical reviewer result.

An unavailable optional source diagnostic is not an unsupported primary
operation. When the primary operation is otherwise authorized, the agent MUST
continue it and preserve the diagnostic as `unsupported` or `unknown`. It MUST
NOT turn it into `PASS` or a permission barrier. An unsupported primary request
or missing required safety contract remains blocked on that specific boundary.
