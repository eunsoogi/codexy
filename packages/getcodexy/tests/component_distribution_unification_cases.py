"""Installed component discovery checks for the unified skill."""

from packages.getcodexy.tests import component_distribution_support as support


class ComponentDistributionUnificationCases:
    def test_installed_cli_rejects_incomplete_core_and_github_surfaces(self) -> None:
        self._run("install", "core")
        self.assertEqual(
            support.health_states(self._run("doctor")), {"core": "healthy"}
        )
        skill = self.marketplace / "plugins/codexy/skills/self-improvement/SKILL.md"
        skill.unlink()
        self.assertEqual(support.health_states(self._run("doctor"))["core"], "stale")
        skill.write_text("x")
        self.assertEqual(
            support.health_states(self._run("doctor"))["core"], "incompatible"
        )

    def test_core_package_has_one_recovery_and_memory_entrypoint(self) -> None:
        self._run("install", "core")
        skills = self.marketplace / "plugins/codexy/skills"
        names = {path.parent.name for path in skills.glob("*/SKILL.md")}
        self.assertIn("self-improvement", names)
        self.assertTrue({"dreaming", "wiki"}.isdisjoint(names))
        self.assertTrue((skills / "self-improvement/scripts/wiki_store.py").is_file())
