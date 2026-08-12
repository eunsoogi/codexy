"""Owned repository merge and auto-merge prevention."""

from .envelope import Request


def forbidden(request: Request) -> bool:
    del request
    return True
