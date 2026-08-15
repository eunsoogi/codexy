"""Shared workload contract constants and output patterns."""

import re

BUDGET_SECONDS = 300.0
MINIMUM_PASSED_TESTS = 1802
REQUIRED_JOB_TIMEOUT_MINUTES = 6  # 300s workload plus 60s setup/cleanup headroom
WORKLOAD = ("cargo", "test", "--locked", "--all-targets")
COMPILE_PATTERN = re.compile(r"Finished `test` profile .* in (?:(\d+)m )?([0-9.]+)s")
RESULT_PATTERN = re.compile(
    r"test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored; \d+ measured; \d+ filtered out"
)
