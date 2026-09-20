"""Errors for selected batch-result application."""


class ApplyError(ValueError):
    """The selected result cannot be applied safely."""


class ApplyInterrupted(ApplyError):
    """Application stopped before the current item was replaced."""
