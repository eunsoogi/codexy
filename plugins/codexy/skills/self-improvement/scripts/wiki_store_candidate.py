"""Validate the shape and safe paths of a Wiki revision candidate."""

from wiki_store_support import digest, encoded, permitted


def inspect_candidate(candidate: dict) -> tuple:
    required = {
        "id",
        "sources",
        "changes",
        "operation",
        "checkpoint",
    }
    if (
        not isinstance(candidate, dict)
        or not required.issubset(candidate)
        or (set(candidate) - required - {"brief_budget_bytes"})
    ):
        raise ValueError("invalid candidate shape")
    identifier, sources = candidate["id"], candidate["sources"]
    changes, operation, checkpoint = (
        candidate["changes"],
        candidate["operation"],
        candidate["checkpoint"],
    )
    if (
        not isinstance(identifier, str)
        or not identifier.strip()
        or not isinstance(sources, list)
        or not sources
        or any(not isinstance(s, str) or not s.strip() or "\n" in s for s in sources)
        or not isinstance(operation, str)
        or not operation.strip()
        or "\n" in operation
        or not isinstance(checkpoint, str)
        or not checkpoint.strip()
        or not isinstance(changes, dict)
        or "_index.md" not in changes
    ):
        raise ValueError(
            "candidate requires identity, evidence, index, article and checkpoint"
        )
    if any(
        not isinstance(k, str) or not permitted(k) or not isinstance(v, str)
        for k, v in changes.items()
    ):
        raise ValueError("candidate contains an unsafe or unsupported path")
    if not any(k.startswith("wiki/") for k in changes):
        raise ValueError("candidate must contain an article")
    candidate_hash = digest(encoded(candidate))
    return identifier, sources, changes, operation, checkpoint, candidate_hash
