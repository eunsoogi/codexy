"""Shared aggregate values for bounded profiler command intervals."""

from __future__ import annotations


def aggregate(
    target: str, key: str, family: str, producers: dict[str, list[tuple[int, int]]]
) -> dict:
    cumulative = sum(
        end - start for intervals in producers.values() for start, end in intervals
    )
    conservative = max(
        (union(intervals) for intervals in producers.values()), default=0
    )
    return {
        "target": target,
        "key": key,
        "family": family,
        "count": sum(len(intervals) for intervals in producers.values()),
        "producer_count": len(producers),
        "cumulative_wait_seconds": round(cumulative / 1_000_000_000, 6),
        "conservative_union_occupancy_seconds": round(conservative / 1_000_000_000, 6),
        "overlap_ratio_upper_bound": round(1 - conservative / cumulative, 6)
        if cumulative
        else 0.0,
    }


def union(intervals: list[tuple[int, int]]) -> int:
    total = 0
    end = None
    for start, current_end in sorted(intervals):
        if end is None or start > end:
            total += current_end - start
            end = current_end
        elif current_end > end:
            total += current_end - end
            end = current_end
    return total
