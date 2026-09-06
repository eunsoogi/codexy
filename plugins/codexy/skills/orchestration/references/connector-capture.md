# Authenticated GitHub connector capture

The supported connector capture uses `provider: "github"` and
`method: "connector"`, with `authenticated: true` and a `source` object
containing the exact `mcp__codex_apps__github_get_pr_info` tool, its
`repository_full_name`/`pr_number` arguments, and the raw connector result. The
canonical producer MUST derive repository, PR number, URL, base branch, base
OID, and head OID from those observed arguments and result fields, populate only
absent derived snapshot fields, preserve extra raw result fields, and reject
contradictory caller-supplied fields. This provenance MUST NOT infer whether an
opaque connector used REST or GraphQL, a capture timestamp, or authentication
from metadata; the host-authorized read and authenticated owning-issue linkage
remain the live proof.
