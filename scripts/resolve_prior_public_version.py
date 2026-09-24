#!/usr/bin/env python3
"""Resolve the highest stable public release below an activation target."""

import re
import subprocess
import sys

VERSION_PATTERN = re.compile(r"v?(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\Z")
MAX_COMPONENT = 2_147_483_647
PUBLIC_RELEASE_QUERY = ".[] | select(.draft == false and .prerelease == false) | .tag_name"
Version = tuple[tuple[int, int, int], str]


def parse_version(value: str) -> Version | None:
    match = VERSION_PATTERN.fullmatch(value)
    if match is None:
        return None
    components = tuple(int(component) for component in match.groups())
    if any(component > MAX_COMPONENT for component in components):
        return None
    return components, ".".join(match.groups())


def public_release_tags(repository: str) -> list[str]:
    try:
        result = subprocess.run(
            [
                "gh",
                "api",
                "--paginate",
                f"repos/{repository}/releases?per_page=100",
                "--jq",
                PUBLIC_RELEASE_QUERY,
            ],
            check=True,
            capture_output=True,
            text=True,
        )
    except (OSError, subprocess.CalledProcessError) as error:
        detail = getattr(error, "stderr", "")
        print(
            "could not read public releases to select the package baseline; "
            f"refusing to dispatch python-package.yml ({detail.strip() or error})",
            file=sys.stderr,
        )
        raise SystemExit(1) from error
    return result.stdout.splitlines()


def resolve(target: Version, target_text: str, release_tags: list[str]) -> str:
    candidates = []
    for tag in release_tags:
        parsed = parse_version(tag.strip())
        if parsed is not None and parsed[0] < target[0]:
            candidates.append(parsed)
    if not candidates:
        raise ValueError(
            f"no published stable release precedes target v{target_text}; "
            "check the public release list and stable version tags"
        )
    return max(candidates, key=lambda candidate: candidate[0])[1]


def main() -> int:
    if len(sys.argv) != 3:
        print(
            "usage: resolve_prior_public_version.py <repository> <target-version>",
            file=sys.stderr,
        )
        return 2
    target_text = sys.argv[2]
    target = parse_version(target_text)
    if target is None:
        print(
            f"target v{target_text} is not a supported MAJOR.MINOR.PATCH version",
            file=sys.stderr,
        )
        return 1
    try:
        print(resolve(target, target_text, public_release_tags(sys.argv[1])))
    except ValueError as error:
        print(error, file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
