"""Shared constants for the local batch-change resume workflow."""

RESUME_SCHEMA = "codexy.batch-change-resume.v1"
DEFAULT_STATE_DIRECTORY = ".codexy-batch-state"
DEFAULT_RESULTS_DIRECTORY = ".codexy-batch-results"
ITEM_STATUSES = {"pending", "in-progress", "succeeded", "failed", "conflict"}
RESOLUTION_VALUES = {"reuse", "rerun", "conflict", "pending"}
