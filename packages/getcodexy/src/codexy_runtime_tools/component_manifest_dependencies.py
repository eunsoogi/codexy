"""Dependency graph operations for component manifests."""

from __future__ import annotations

from itertools import combinations
from typing import Protocol


class ComponentLike(Protocol):
    id: str
    dependencies: tuple[str, ...]


def compatible_combinations(
    components: tuple[ComponentLike, ...],
) -> set[tuple[str, ...]]:
    ids, dependencies = (
        tuple(component.id for component in components),
        {component.id: set(component.dependencies) for component in components},
    )
    return {
        subset
        for size in range(len(ids) + 1)
        for subset in combinations(ids, size)
        if all(dependencies[item].issubset(subset) for item in subset)
    }


def has_cycle(components: tuple[ComponentLike, ...]) -> bool:
    dependencies = {component.id: component.dependencies for component in components}
    visiting, visited = set(), set()

    def visit(component: str) -> bool:
        if component in visiting:
            return True
        if component in visited:
            return False
        visiting.add(component)
        cyclic = any(visit(dependency) for dependency in dependencies[component])
        visiting.remove(component)
        visited.add(component)
        return cyclic

    return any(visit(component) for component in dependencies)
