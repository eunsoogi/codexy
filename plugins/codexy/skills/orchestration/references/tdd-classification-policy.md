# TDD classification policy

The packaged validator accepts two request versions. v1 remains compatible with
the four-field result: `classification`, `engineering_tdd_required`,
`tdd_boundaries`, and `proportional_proof_boundaries`. In v1,
`engineering_tdd_required` is the legacy engineering-boundary signal; it is not
the v2 sequencing decision.

## v2 request

Callers MUST use `codexy.tdd-classification-request.v2` when they can describe
each changed boundary. Each boundary has a unique `id`, a known surface `kind`,
`change.purpose`, a closed `risks` list, and explicit `test_first_required`.
Repeated kinds are allowed when their ids and responsibilities differ. The
`kind` identifies the changed surface, not a filename or an inferred purpose.
Known engineering kinds are `production_code`, `runtime_behavior`, `validator`,
`parser`, `markdown_backed_parser`, `hook`, `cli`, `workflow`, `installer`,
`package_resolution`, `tool_behavior`, and `executable_contract`. Known
non-engineering kinds are `readme`, `documentation`, `instruction_only_skill`,
`agent_prompt`, `declarative_metadata`, `issue_or_pr_metadata`,
`roadmap_or_release_prose`, `inventory`, `diagram`, `example`, and `copy_edit`.
`defect_repair` and `behavior_preserving_refactor` are purposes, not kinds.

The supported purposes are `feature`, `defect_repair`,
`behavior_preserving_refactor`, and `instruction_only`. A defect repair MUST
state `change.reproduction.status` as `available` or `unavailable`. An
unavailable reproduction MUST include a non-empty `reason` and `alternative`; an
available reproduction MUST NOT include either. Risks MUST be limited to
`permission`, `destructive_state`, and `recovery`.

## v2 result

The v2 result is `codexy.tdd-classification-result.v2`. The top-level
`engineering_tests_required` flag means every engineering boundary needs
requirement-linked behavioral verification. `tdd_mode` is a conservative task
summary: `required` if any boundary requires test-first sequencing, `optional`
if an engineering boundary has no such mandate, and `not_applicable` when all
boundaries are non-engineering. In a mixed request, callers MUST use each
`boundary_obligations` entry rather than applying the summary mode to every
boundary.

Each obligation identifies the boundary, repeats `engineering_tests_required`
and `tdd_mode`, and lists `pre_change_obligations` and `proof_obligations`.
Engineering entries use `requirement_linked_behavioral_verification`; other
entries use `proportional_structural_or_behavioral_proof`.

| Boundary fact                                   | Behavioral tests                  | Sequencing and pre-change duty                                                                   |
| ----------------------------------------------- | --------------------------------- | ------------------------------------------------------------------------------------------------ |
| ordinary feature                                | required                          | `optional` unless explicitly mandated                                                            |
| reproducible engineering defect                 | required                          | `required` and `faithful_red_before_fix`                                                         |
| unavailable engineering defect reproduction     | required                          | record `justified_alternative_before_change`; no manufactured RED                                |
| behavior-preserving refactor                    | required                          | `optional` with `green_or_characterization_baseline`                                             |
| permission, destructive-state, or recovery risk | required for engineering surfaces | establish the matching invariants before implementation; do not manufacture RED for every helper |
| instruction-only or other non-engineering work  | not required                      | `not_applicable` plus proportional proof, including documentation defects                        |

Mixed requests retain these duties independently: engineering boundaries keep
behavioral verification, non-engineering boundaries keep proportional proof even
when their purpose is defect repair, and only their own explicit or
defect-driven sequencing mode applies.

Empty boundaries, duplicate ids or risks, unknown kinds or enum values,
malformed facts, unknown fields, and unsupported schema versions are rejected.
Uncertainty MUST NOT lower a behavioral-test obligation. The executable boundary
sets and schema validation are maintained by the packaged runtime validator.
