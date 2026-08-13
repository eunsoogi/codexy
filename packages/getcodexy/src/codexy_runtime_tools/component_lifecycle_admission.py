"""Pre-mutation inventory and operation receipt admission for lifecycle commands."""

from __future__ import annotations

from pathlib import Path

from .component_manifest import ComponentManifest
from .component_resolver import ComponentResolutionError, admit_installed_inventory, canonical_components, resolve_components
from .component_transaction_receipts import read_receipt
from .component_transaction_state import Journal


def admitted_selection(manifest: ComponentManifest, inventory: object, marketplace_root: Path | None) -> tuple[str, ...]:
    return admit_installed_inventory(manifest, inventory, marketplace_root)


def replay_receipt(home: Path, manifest: ComponentManifest, identifier: str, command: str, requested: tuple[str, ...]) -> dict[str, object] | None:
    receipt = read_receipt(home, identifier)
    if receipt is None:
        return None
    _validate_receipt(manifest, receipt)
    if receipt["operation_id"] == identifier and receipt["command"] == command and receipt["requested_components"] == list(requested):
        return receipt
    raise ValueError(f"operation receipt conflicts with requested operation: {identifier}")


def admit_pending_receipt(home: Path, manifest: ComponentManifest, journal: Journal) -> dict[str, object] | None:
    receipt = read_receipt(home, journal.identifier)
    if receipt is None:
        return None
    _validate_receipt(manifest, receipt)
    if receipt["operation_id"] != journal.identifier or receipt["command"] != journal.command or receipt["requested_components"] != list(journal.requested) or receipt["resolved_components"] != list(journal.resolved) or receipt["selection_before"] != list(journal.before):
        raise ValueError(f"pending transaction receipt conflicts with journal: {journal.identifier}")
    if journal.phase == "committed" and receipt["outcome"] == "completed" and receipt["selection_after"] == list(journal.target):
        return receipt
    if journal.phase == "rolling-back" and receipt["outcome"] == "rolled-back" and receipt["selection_after"] == list(journal.before):
        return receipt
    raise ValueError(f"pending transaction receipt conflicts with journal: {journal.identifier}")


def matching_receipt(home: Path, manifest: ComponentManifest, receipt: dict[str, object]) -> bool:
    identifier = receipt["operation_id"]
    existing = read_receipt(home, identifier) if isinstance(identifier, str) else None
    if existing is not None:
        _validate_receipt(manifest, existing)
    return existing == receipt


def _validate_receipt(manifest: ComponentManifest, receipt: dict[str, object]) -> None:
    before, after, resolved = (_components(receipt, field) for field in ("selection_before", "selection_after", "resolved_components"))
    requested = _components(receipt, "requested_components")
    outcome, errors = receipt["outcome"], receipt["errors"]
    if before not in manifest.compatible_combinations or after not in manifest.compatible_combinations:
        raise ValueError("operation receipt has invalid component selections")
    if outcome == "rejected":
        if resolved or after != before or not _single_domain_error(manifest, errors):
            raise ValueError("operation receipt has invalid rejection semantics")
        return
    try:
        expected_resolved, expected_target = _operation_plan(manifest, str(receipt["command"]), requested, before)
    except ComponentResolutionError as error:
        raise ValueError("operation receipt has an invalid request contract") from error
    if resolved != expected_resolved:
        raise ValueError("operation receipt has an invalid resolved selection")
    if outcome == "completed" and (after != expected_target or errors != []):
        raise ValueError("operation receipt has invalid completion semantics")
    if outcome == "rolled-back" and (after != before or errors != [{"code": "operation-failed"}]):
        raise ValueError("operation receipt has invalid rollback semantics")


def _components(receipt: dict[str, object], field: str) -> tuple[str, ...]:
    value = receipt[field]
    if not isinstance(value, list) or any(not isinstance(component, str) for component in value):
        raise ValueError("operation receipt has invalid components")
    return tuple(value)


def _single_domain_error(manifest: ComponentManifest, errors: object) -> bool:
    return isinstance(errors, list) and len(errors) == 1 and isinstance(errors[0], dict) and errors[0].get("code") in manifest.domain_errors


def _operation_plan(manifest: ComponentManifest, command: str, requested: tuple[str, ...], before: tuple[str, ...]) -> tuple[tuple[str, ...], tuple[str, ...]]:
    if command == "install":
        resolved = resolve_components(manifest, requested)
        return resolved, canonical_components(manifest, set(before) | set(resolved))
    if command == "update":
        resolved = before if not requested else resolve_components(manifest, requested)
        if not set(resolved).issubset(before):
            raise ComponentResolutionError("incompatible-component-selection")
        return resolved, before
    if not requested:
        raise ComponentResolutionError("missing-removal-target")
    resolve_components(manifest, requested)
    resolved = canonical_components(manifest, set(requested))
    target = canonical_components(manifest, set(before) - set(resolved))
    if target not in manifest.compatible_combinations:
        raise ComponentResolutionError("dependency-protected-removal")
    return resolved, target
