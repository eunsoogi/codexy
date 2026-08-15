"""Cargo test-output patterns shared by profiler accounting modules."""

from __future__ import annotations

import re

LIST_PATTERN = re.compile(r"^(?P<name>.+): (?:test|benchmark)$")
RUN_PATTERN = re.compile(
    r"^test (?P<name>.+) \.\.\. (?P<result>ok|FAILED|ignored)(?=$|[A-Z])"
)
RUN_START_PATTERN = re.compile(r"^test (?P<name>.+?) \.\.\. .+$")
RUNNING_NOTICE_PATTERN = re.compile(
    r"^test (?P<name>.+) has been running for over 60 seconds$"
)
RUNNING_BINARY_PATTERN = re.compile(r"^\s*Running .+ \((?P<binary>.+)\)$")
RESULT_SUMMARY_PATTERN = re.compile(r"^test result: (?:ok|FAILED)\.")
RESULT_COUNTS_PATTERN = re.compile(
    r"^test result: (?:ok|FAILED)\. (?P<passed>\d+) passed; (?P<failed>\d+) failed; (?P<ignored>\d+) ignored;"
)
