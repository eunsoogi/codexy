"""Operation receipt fixture builder."""


def make_receipt(
    identifier: str,
    command: str,
    requested: tuple[str, ...],
    resolved: tuple[str, ...],
    before: tuple[str, ...],
    after: tuple[str, ...],
    outcome: str,
    errors: list[dict[str, str]] | None = None,
) -> dict[str, object]:
    return {
        "schema": "getcodexy.operation-receipt.v1",
        "operation_id": identifier,
        "command": command,
        "outcome": outcome,
        "requested_components": list(requested),
        "resolved_components": list(resolved),
        "selection_before": list(before),
        "selection_after": list(after),
        "installed_components": list(after),
        "source_of_truth": "installed-component-inventory",
        "errors": [] if errors is None else errors,
    }
