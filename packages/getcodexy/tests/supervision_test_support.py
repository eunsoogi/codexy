"""Load the core policy under an isolated name alongside other plugins."""

import importlib
import importlib.util
import sys
from pathlib import Path

POLICY_ROOT = Path(__file__).resolve().parents[3] / "plugins/codexy/hooks/codexy_policy"
PACKAGE = "supervision_test_policy"
previous_bytecode_setting = sys.dont_write_bytecode
sys.dont_write_bytecode = True
if PACKAGE not in sys.modules:
    spec = importlib.util.spec_from_file_location(
        PACKAGE,
        POLICY_ROOT / "__init__.py",
        submodule_search_locations=[str(POLICY_ROOT)],
    )
    assert spec and spec.loader
    package = importlib.util.module_from_spec(spec)
    sys.modules[PACKAGE] = package
    spec.loader.exec_module(package)

reservations = importlib.import_module(f"{PACKAGE}.supervision_reservations")
dispatch = importlib.import_module(f"{PACKAGE}.supervision_dispatch")
observation = importlib.import_module(f"{PACKAGE}.supervision_observation")
reconciliation = importlib.import_module(f"{PACKAGE}.supervision_reconciliation")
resume = importlib.import_module(f"{PACKAGE}.supervision_resume")
storage = importlib.import_module(f"{PACKAGE}.supervision_storage")
sys.dont_write_bytecode = previous_bytecode_setting
