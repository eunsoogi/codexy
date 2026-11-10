#!/usr/bin/env python3
"""Verify the packaged bridge and export a separate rollback replay."""

from __future__ import annotations

import argparse
import subprocess
from pathlib import Path

from resumable_context_capsule import (
    absolute,
    default_runtime_root,
    load_manifest,
    safe_ancestors,
    selected_bridge,
)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--replay", required=True, type=Path)
    parser.add_argument("--authority", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--runtime-root", type=Path)
    args = parser.parse_args()
    plugin_root = Path(__file__).resolve().parents[3]
    root = (
        absolute(args.runtime_root)
        if args.runtime_root
        else default_runtime_root(plugin_root)
    )
    safe_ancestors(root, directory=True, label="runtime")
    safe_ancestors(args.replay, directory=False, label="replay")
    safe_ancestors(args.authority, directory=False, label="authority")
    safe_ancestors(args.output.parent, directory=True, label="output")
    platforms, _ = load_manifest(root)
    bridge = selected_bridge(root, platforms)
    # All three paths remain explicit; this never replaces the source replay.
    result = subprocess.run(
        [
            bridge,
            "--export-previous-replay",
            "--replay",
            absolute(args.replay),
            "--authority",
            absolute(args.authority),
            "--output",
            absolute(args.output),
        ],
        check=False,
    )
    return result.returncode


if __name__ == "__main__":
    raise SystemExit(main())
