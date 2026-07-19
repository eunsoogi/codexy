from __future__ import annotations

import json
from pathlib import Path

from codexy_runtime_tools import runtime
from codexy_runtime_tools.cache import runtime_cache_key


def configuration(root: Path, **overrides: object) -> runtime.Configuration:
    plugin_root = root / "plugin root 유니코드"
    manifest = plugin_root / ".codex-plugin" / "plugin.json"
    manifest.parent.mkdir(parents=True, exist_ok=True)
    manifest.write_text(json.dumps({"version": "1.2.1"}), encoding="utf-8")
    values: dict[str, object] = {
        "server": "lsp",
        "plugin_root": plugin_root,
        "arguments": ["--stdio"],
        "platform": "linux-x86_64",
        "manifest": manifest,
        "release": "1.2.1",
        "runtime_name": "codexy-mcp-lsp-linux-x86_64.bin",
        "package_path": "",
        "package_url": "https://example.test/package.tar.gz",
        "artifacts_api": "",
        "package_override": False,
        "package_sha256": "",
        "git_repository": "https://example.test/codexy.git",
        "git_ref": "a" * 40,
        "offline": False,
        "git_fallback": False,
    }
    values.update(overrides)
    return runtime.Configuration(**values)  # type: ignore[arg-type]


def install_paths(config: runtime.Configuration, cache: Path) -> tuple[Path, Path]:
    source = (
        "\n".join(
            (
                "package-override",
                config.package_path,
                config.package_url,
                config.artifacts_api,
                config.package_sha256,
            )
        )
        if config.package_override
        else "package-default"
    )
    key = runtime_cache_key(
        manifest=config.manifest,
        package_override=config.package_override,
        identity=[
            config.git_repository,
            config.git_ref,
            config.platform,
            runtime.PROTOCOL,
            source,
            f"codexy-mcp-{config.server}",
        ],
    )
    root = cache / key
    return root / "bin" / f"codexy-mcp-{config.server}", root / "plugin.json"
