"""Exact Python equivalents of the canonical Rust title validators."""


def _commit_type(value: str) -> bool:
    return bool(value) and all(char.isascii() and (char.islower() or char.isdigit()) or char == "-" for char in value)


def _scope(value: str) -> bool:
    return bool(value) and all(char.isascii() and (char.islower() or char.isdigit()) or char in "-_/" for char in value)


def _prefix(value: str) -> bool:
    value = value.removesuffix("!")
    if "(" not in value:
        return _commit_type(value)
    commit_type, scope = value.split("(", 1)
    return scope.endswith(")") and _commit_type(commit_type) and _scope(scope[:-1])


def pr_title(value: object) -> bool:
    if not isinstance(value, str) or ": " not in value:
        return False
    prefix, summary = value.split(": ", 1)
    return bool(summary.strip()) and _prefix(prefix)


def issue_title(value: object) -> bool:
    if not isinstance(value, str) or not value or not value[0].isascii() or not value[0].isupper():
        return False
    return not _issue_conventional(value)


def _issue_conventional(value: str) -> bool:
    if ": " in value:
        prefix, summary = value.split(": ", 1)
        if summary.strip():
            lowered = prefix.lower()
            if _prefix(lowered) or (prefix.endswith(":") and _prefix(prefix[:-1].lower())):
                return True
            token = prefix.split()[0] if prefix.split() else prefix
            if ("(" in token or token.endswith("!")) and _prefix(token.lower()):
                return True
    token = value.split()[0] if value.split() else value
    prefix = token.split(":", 1)[0].removesuffix(":")
    return ("(" in prefix or prefix.endswith("!") or ":" in token) and _prefix(prefix.lower())
