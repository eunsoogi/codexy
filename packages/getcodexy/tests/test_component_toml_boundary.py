from __future__ import annotations

import unittest

from codexy_runtime_tools.component_diagnostic_surfaces import SurfaceDiagnosis, diagnose_surface
from codexy_runtime_tools.component_inspection import doctor
from codexy_runtime_tools.component_source_admission import DiagnosticFailure, DiagnosticTree

from component_lifecycle_support import fixture
from test_component_inspection import materialize


class ComponentTomlBoundaryTests(unittest.TestCase):
    def test_oversized_signed_integers_are_incompatible_not_process_failures(self) -> None:
        for sign in ("", "-"):
            with self.subTest(sign=sign):
                result = _doctor('extra = ' + sign + '9' * 5_000)
                self.assertEqual(_health(result), _incompatible())

    def test_nested_arrays_and_tables_at_and_above_depth_limit(self) -> None:
        for kind, source in (
            ("array-at-limit", 'extra = ' + '[' * 128 + '0' + ']' * 128),
            ("table-at-limit", '[' + '.'.join(f'v{index}' for index in range(128)) + ']\nvalue = 0'),
        ):
            with self.subTest(kind=kind):
                self.assertEqual(_surface(source), SurfaceDiagnosis(False))
        for kind, source in (
            ("array-over-limit", 'extra = ' + '[' * 129 + '0' + ']' * 129),
            ("table-over-limit", '[' + '.'.join(f'v{index}' for index in range(129)) + ']\nvalue = 0'),
        ):
            with self.subTest(kind=kind):
                self.assertEqual(_surface(source), SurfaceDiagnosis(False, DiagnosticFailure.MALFORMED))

    def test_document_string_and_collection_limits_are_incompatible(self) -> None:
        cases = (
            ("document", '#' * 65_536),
            ("string", 'extra = "' + 'x' * 8_193 + '"'),
            ("collection", 'extra = [' + ','.join('0' for _ in range(1_025)) + ']'),
        )
        for kind, source in cases:
            with self.subTest(kind=kind):
                self.assertEqual(_surface(source), SurfaceDiagnosis(False, DiagnosticFailure.MALFORMED))
                self.assertEqual(_health(_doctor(source)), _incompatible())

    def test_malformed_duplicate_and_conflicting_catalogs_remain_incompatible(self) -> None:
        for source in (
            'version = "0.1.0"\nversion = "0.1.0"',
            'version = { value = "0.1.0" }',
            '[catalog\nvalue = 0',
        ):
            with self.subTest(source=source):
                self.assertEqual(_health(_doctor(source)), _incompatible())

    def test_current_and_older_canonical_catalogs_preserve_health_direction(self) -> None:
        for version, expected in (("1.3.0", "healthy"), ("1.2.0", "stale")):
            with self.subTest(version=version), fixture({"core"}, versions={"core": version}) as state:
                materialize(state, "core", version=version)
                result = doctor(state.home, codex=state.codex, runner=state.run)
            self.assertEqual(_health(result)["state"], expected)


def _surface(source: str) -> SurfaceDiagnosis:
    with fixture({"core"}) as state:
        materialize(state, "core")
        catalog = state.marketplace / "plugins/codexy/agents/catalog.toml"
        catalog.write_text(catalog.read_text(encoding="utf-8") + "\n" + source + "\n", encoding="utf-8")
        return diagnose_surface(DiagnosticTree(state.marketplace / "plugins/codexy"), "core")


def _doctor(source: str) -> dict[str, object]:
    with fixture({"core"}) as state:
        materialize(state, "core")
        catalog = state.marketplace / "plugins/codexy/agents/catalog.toml"
        catalog.write_text(catalog.read_text(encoding="utf-8") + "\n" + source + "\n", encoding="utf-8")
        return doctor(state.home, codex=state.codex, runner=state.run)


def _health(result: dict[str, object]) -> dict[str, str]:
    return result["component_health"][0]  # type: ignore[index]


def _incompatible() -> dict[str, str]:
    return {
        "component": "core",
        "state": "incompatible",
        "repair": "repair the Codexy registration, then rerun getcodexy doctor",
    }


if __name__ == "__main__":
    unittest.main()
