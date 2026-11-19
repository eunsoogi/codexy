"""Load recipient policy without occupying another plugin's module namespace."""

import importlib

from packages.getcodexy.tests import supervision_test_support as supervision

envelope = importlib.import_module(f"{supervision.PACKAGE}.envelope")
delivery = importlib.import_module(f"{supervision.PACKAGE}.thread_delivery")
recipient_store = importlib.import_module(f"{supervision.PACKAGE}.recipient_store")
Dispatch = supervision.dispatch.Dispatch
open_store = supervision.storage.open_store
