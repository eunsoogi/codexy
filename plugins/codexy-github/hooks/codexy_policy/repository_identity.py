"""Canonical repository URL identity parsing."""

from __future__ import annotations

import re
from urllib.parse import urlsplit


SCP = re.compile(r"^(?:[A-Za-z0-9._-]+@)?(?P<host>[A-Za-z0-9.-]+):(?P<path>[^\s?#]+)$")


def identity(url: str) -> tuple[str, str, str] | None:
    match = None if "://" in url else SCP.fullmatch(url)
    if match:
        host, path = match.group("host"), match.group("path")
    else:
        parsed = urlsplit(url)
        if (
            parsed.scheme not in {"http", "https", "ssh", "git"}
            or not parsed.hostname
            or parsed.password
            or parsed.query
            or parsed.fragment
        ):
            return None
        host, path = parsed.hostname, parsed.path.lstrip("/")
    host = host.lower()
    if host != "github.com":
        return host, "", ""
    parts = path.removesuffix(".git").split("/")
    return (
        (host, parts[0].lower(), parts[1].lower())
        if len(parts) == 2 and all(parts)
        else None
    )


def github_identity(value: str) -> tuple[str, str, str] | None:
    if "://" not in value:
        value = (
            "https://" + value
            if value.count("/") == 2
            else "https://github.com/" + value
        )
    return identity(value)
