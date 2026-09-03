#!/usr/bin/env python3
"""Select the exact wheel and sdist from one public PyPI release document."""

import json
import sys
from pathlib import Path
from urllib.parse import urlsplit


def main() -> int:
    if len(sys.argv) != 3:
        raise SystemExit("usage: select_public_getcodexy_artifacts.py JSON VERSION")
    document = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
    version = sys.argv[2]
    if document.get("info", {}).get("version") != version:
        raise SystemExit("public getcodexy version mismatch")
    for package_type in ("bdist_wheel", "sdist"):
        matches = [item for item in document.get("urls", []) if item.get("packagetype") == package_type]
        if len(matches) != 1:
            print(f"expected one public {package_type}, got {len(matches)}", file=sys.stderr)
            return 2
        item = matches[0]
        filename = urlsplit(item["url"]).path.rsplit("/", 1)[-1]
        print(package_type, item["url"], item["digests"]["sha256"], filename, sep="\t")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
