from __future__ import annotations

import unittest

from codexy_runtime_tools.component_json import loads
from codexy_runtime_tools.component_manifest import parse_component_manifest
from codexy_runtime_tools.component_transaction_state import decode_inventory


class StrictComponentJsonTests(unittest.TestCase):
    def test_rejects_nonfinite_constants_at_nested_and_top_level_positions(self) -> None:
        for source in ('NaN', 'Infinity', '-Infinity', '1e999', '-1e999', '{"outer":{"inner":NaN}}', '{"outer":{"inner":-1e999}}'):
            with self.subTest(source=source):
                with self.assertRaisesRegex(ValueError, "non-finite"):
                    loads(source)

    def test_standard_json_and_case_controls_remain_unambiguous(self) -> None:
        self.assertEqual(loads('{"value":[null,true,false,1.5]}'), {"value": [None, True, False, 1.5]})
        for source in ('nan', 'infinity', '-infinity'):
            with self.subTest(source=source):
                with self.assertRaises(ValueError):
                    loads(source)

    def test_manifest_and_durable_inventory_ingestion_reject_nonfinite_values(self) -> None:
        with self.assertRaisesRegex(ValueError, "non-finite"):
            parse_component_manifest('{"schema":NaN}')
        with self.assertRaisesRegex(ValueError, "non-finite"):
            decode_inventory(b'{"schema":"getcodexy.installed-component-inventory.v1","components":[],"nested":{"value":Infinity}}')

    def test_deep_json_is_normalized_to_typed_input_failure(self) -> None:
        with self.assertRaisesRegex(ValueError, "nesting"):
            loads(_nested_array(2_000))
        with self.assertRaisesRegex(ValueError, "nesting"):
            decode_inventory((
                '{"schema":"getcodexy.installed-component-inventory.v1","components":[],"extra":'
                + _nested_array(2_000)
                + "}"
            ).encode())

    def test_reasonable_depth_and_finite_exponent_remain_valid(self) -> None:
        self.assertEqual(_array_depth(loads(_nested_array(32))), 32)
        self.assertEqual(loads("1e99"), 1e99)

    def test_exact_nesting_boundary_accepts_128_and_rejects_129(self) -> None:
        self.assertEqual(_array_depth(loads(_nested_array(128))), 128)
        with self.assertRaisesRegex(ValueError, "nesting"):
            loads(_nested_array(129))


def _nested_array(depth: int) -> str:
    return "[" * depth + "0" + "]" * depth


def _array_depth(value: object) -> int:
    depth = 0
    while isinstance(value, list):
        depth += 1
        value = value[0]
    return depth


if __name__ == "__main__":
    unittest.main()
