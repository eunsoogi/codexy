"""Errors raised by the explicit batch-change resume workflow."""


class ResumeError(ValueError):
    """The explicit resume request cannot safely continue."""
