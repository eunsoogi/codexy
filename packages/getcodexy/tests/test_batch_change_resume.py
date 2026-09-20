"""Discovery entrypoint for the batch-change resume scenario modules."""

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))

from batch_change_resume_cases.batch_abort import BatchChangeResumeBatchAbortTests
from batch_change_resume_cases.concurrency import BatchChangeResumeConcurrencyTests
from batch_change_resume_cases.interruptions import BatchChangeResumeInterruptionTests
from batch_change_resume_cases.persistence import BatchChangeResumePersistenceTests
from batch_change_resume_cases.reuse import BatchChangeResumeReuseTests

__all__ = [
    "BatchChangeResumeBatchAbortTests",
    "BatchChangeResumeConcurrencyTests",
    "BatchChangeResumeInterruptionTests",
    "BatchChangeResumePersistenceTests",
    "BatchChangeResumeReuseTests",
]


if __name__ == "__main__":
    import unittest

    unittest.main()
