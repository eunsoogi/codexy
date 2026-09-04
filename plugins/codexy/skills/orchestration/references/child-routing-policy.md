# Child routing policy

Named packaged specialists are selected first and caller model overrides are
forbidden. Generic work defaults to `gpt-5.6-luna` at `max`; when that route is
unavailable, it falls back to `gpt-5.6-terra` at `high`. Simple work uses the
same Luna route when all simple predicates are complete.

Thread delivery MUST bind `model` and `thinking` to the authenticated recipient,
not copy the sender settings. Parent-to-generic-child delivery MUST use
`gpt-5.6-luna` at `max`; child-to-root delivery MUST use `gpt-5.6-sol` at
`medium`. Both fields MUST be explicit. Unsupported or mismatched recipient
settings MUST fail closed instead of falling back to the sender route.

Control-plane receipts and status handoffs MUST carry a stable `transition key`,
`event id`, or `state fingerprint` as applicable. Pre-delivery and post-result
receipts MUST use `transition key`. A completed delivery with the same
recipient, phase, and key MUST NOT be sent again; unchanged status MUST remain
silent.

Unknown, ambiguous, incomplete, and unsupported requests fail closed to the root
or named-specialist route. The executable contract is maintained by the packaged
runtime validator.
