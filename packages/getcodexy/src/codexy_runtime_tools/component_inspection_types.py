"""Typed internal contracts for component inspection."""

from typing import TypedDict

from .component_manifest import ComponentManifest


class InspectionReport(TypedDict):
    manifest: ComponentManifest
    actual: tuple[str, ...]
    recorded: tuple[str, ...] | None
    records: dict[str, dict[str, object]]
    admission_error: str | None
    host_error: str | None
    activation: dict[str, str]
    inventory: dict[str, object]
    consistency: str
    errors: list[dict[str, str]]
