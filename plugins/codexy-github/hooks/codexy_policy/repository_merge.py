"""Owned repository merge and auto-merge prevention."""

from .envelope import Request


def forbidden(_request: Request) -> bool | str:
    return "UNAVAILABLE"
