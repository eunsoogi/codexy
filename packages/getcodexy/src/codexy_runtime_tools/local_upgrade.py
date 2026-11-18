"""Verified local archive switching within the component transaction boundary."""

from __future__ import annotations

from dataclasses import replace
from pathlib import Path

from .component_lifecycle_preflight import Runner
from .component_manifest import ComponentManifest
from .component_transition_journal import Journal
from .component_transaction_snapshot import InventorySnapshot
from .local_marketplace import archive_version
from .local_upgrade_host import admit_partial, switch
from .local_upgrade_state import LocalUpgrade, read, tree_digest
from .plugin_resolution import MarketplaceBinding, MarketplaceIdentity


def local_version(root: MarketplaceBinding, manifest: ComponentManifest) -> str | None:
    if not isinstance(root, MarketplaceIdentity) or root.source_type != "local":
        return None
    return archive_version(root.root, tuple(c.plugin for c in manifest.components))


def forward(
    home: Path,
    executable: Path,
    invoke: Runner,
    manifest: ComponentManifest,
    root: MarketplaceBinding,
    journal: Journal,
) -> MarketplaceBinding:
    version = local_version(root, manifest)
    if version is None or version == manifest.version:
        return root
    from .local_release_bundle import acquire_target

    # The core journal is already durable. No host mutation occurs before both
    # archives and their operation-bound identity have been recorded.
    state = read(home)
    if state is None:
        before_digest = tree_digest(root.root)
        target = acquire_target(
            home, manifest.version, tuple(c.plugin for c in manifest.components)
        )
        from .component_lifecycle_mcp import materialize_mcp_sources

        materialize_mcp_sources(target, manifest, journal.target)
        state = LocalUpgrade(
            journal.identifier,
            root,
            target,
            version,
            manifest.version,
            before_digest,
            tree_digest(target.root),
        )
        state.validate(home, journal, manifest)
        state.write(home)
    state.validate(home, journal, manifest)
    admit_partial(executable, invoke, manifest, journal, state)
    switch(executable, invoke, state.target, (state.old, state.target))
    return state.target


def rollback(
    home: Path,
    executable: Path,
    invoke: Runner,
    manifest: ComponentManifest,
    root: MarketplaceBinding,
    journal: Journal,
) -> bool:
    from .component_lifecycle_recovery import list_installed, mutate
    from .component_lifecycle_mcp import materialize_mcp_caches
    from .component_resolver import verify_post_operation_inventory

    state = read(home)
    version = state.old_version if state else local_version(root, manifest)
    if version is None or version == manifest.version:
        return False
    old = state.old if state else root
    old_manifest = replace(
        manifest,
        version=version,
        components=tuple(replace(c, version=version) for c in manifest.components),
    )
    if state is not None:
        state.validate(home, journal, manifest, target_contents=False)
        current = admit_partial(executable, invoke, manifest, journal, state)
        switch(executable, invoke, old, (state.old, state.target))
        # Reinstall every original component: a failed add can have changed its
        # version even while the selected component ID stayed the same.
        for component in journal.before:
            mutate(executable, invoke, "add", old_manifest, component)
        for component in reversed(current):
            if component not in journal.before:
                mutate(executable, invoke, "remove", old_manifest, component)
        materialize_mcp_caches(home, old, old_manifest, journal.before)
        state.validate(home, journal, manifest, target_contents=False)
    verify_post_operation_inventory(
        old_manifest, list_installed(executable, invoke), journal.before, old
    )
    journal.snapshot.restore(home)
    if InventorySnapshot.capture(home) != journal.snapshot:
        raise RuntimeError("local upgrade snapshot restoration did not match")
    return True


def recover_pending(
    home: Path,
    executable: Path,
    invoke: Runner,
    manifest: ComponentManifest,
    journal: Journal | None,
) -> bool:
    """Interrupted local switching rolls back before ordinary inventory admission."""
    state = read(home)
    if state is None:
        return False
    if journal is None:
        raise ValueError("orphaned local upgrade identity requires durable recovery")
    state.validate(
        home, journal, manifest, target_contents=journal.phase == "committed"
    )
    if journal.phase == "committed":
        return False
    from .component_lifecycle_recovery import rollback_or_raise
    from .component_lifecycle_terminal import terminal
    from .component_transaction_state import clear_journal

    from .component_transaction_state import clear_stale_registration_lock

    clear_stale_registration_lock(home)
    rollback_or_raise(
        home,
        executable,
        invoke,
        manifest,
        state.old,
        journal,
        RuntimeError("interrupted local marketplace upgrade"),
    )
    terminal(home, manifest, journal.receipt("rolled-back", journal.before))
    clear_journal(home)
    return True


def refresh(
    home: Path,
    executable: Path,
    invoke: Runner,
    manifest: ComponentManifest,
    root: MarketplaceBinding,
    journal: Journal,
) -> MarketplaceBinding:
    from .component_lifecycle_preflight import refresh_operation_root

    state = read(home)
    if state:
        state.validate(home, journal, manifest, target_contents=False)
        admit_partial(executable, invoke, manifest, journal, state)
        return state.old
    if local_version(root, manifest) not in (None, manifest.version):
        from .component_lifecycle_preflight import existing_marketplace

        if (
            existing_marketplace(executable, invoke, manifest, allow_older_local=True)
            != root
        ):
            raise ValueError("prior local marketplace identity changed")
        return root
    return refresh_operation_root(
        executable, invoke, manifest, root, home, journal.command
    )


def pending_state(
    home: Path, executable: Path, invoke: Runner, manifest: ComponentManifest
):
    from .component_lifecycle_admission import admit_pending_receipt
    from .component_transaction_state import read_journal, decode_inventory

    pending = read_journal(home)
    if pending:
        pending.validate(manifest, decode_inventory)
    receipt = admit_pending_receipt(home, manifest, pending) if pending else None
    if recover_pending(home, executable, invoke, manifest, pending):
        pending, receipt = None, None
    return pending, receipt


def verify_target(home: Path, journal: Journal, manifest: ComponentManifest) -> None:
    state = read(home)
    if state is not None:
        state.validate(home, journal, manifest)


def validate_inventory_version(
    manifest: ComponentManifest, inventory: object, root: MarketplaceBinding | None
) -> None:
    from .component_inventory_classification import (
        classify_installed_inventory,
        InstalledIdentity,
    )
    from .component_resolver import ComponentResolutionError

    if root is None or (version := local_version(root, manifest)) is None:
        return
    for record in classify_installed_inventory(manifest, inventory).records:
        if (
            record.identity is InstalledIdentity.KNOWN
            and record.entry["version"] != version
        ):
            raise ComponentResolutionError("component-version-mismatch")
