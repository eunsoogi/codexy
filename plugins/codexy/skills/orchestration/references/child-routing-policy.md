# Child routing policy

Named packaged specialists are selected first and caller model overrides are
forbidden. Generic work defaults to `gpt-5.6-luna` at `max`; when that route is
unavailable, it falls back to `gpt-5.6-terra` at `high`. Simple work uses the
same Luna route when all simple predicates are complete. Child-to-root
delivery uses `gpt-5.6-sol` at `medium` when supported.

Unknown, ambiguous, incomplete, and unsupported requests fail closed to the
root or named-specialist route. The executable contract is maintained by the
packaged runtime validator.
