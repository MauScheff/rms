import pathlib
import sys
import unittest

sys.path.insert(0, str(pathlib.Path(__file__).parents[1] / "src"))

from example_python.representation import accept_command, make_label, reject_command
from example_python.transition import (
    generate_malformed_input_cases,
    generate_property_cases,
    transition,
)


def run_transition_property() -> None:
    for case in generate_property_cases():
        label = make_label(case)
        assert label is not None
        assert transition(accept_command(label)).rejection is None
        assert transition(reject_command(label)).rejection is not None


def run_malformed_input_fuzz() -> None:
    cases = generate_malformed_input_cases()
    assert len(cases) >= 64
    for raw in cases:
        assert make_label(raw) is None


class BindingTests(unittest.TestCase):
    def test_accepts_valid_label(self) -> None:
        label = make_label("widget")
        self.assertIsNotNone(label)
        output = transition(accept_command(label))
        self.assertIsNone(output.rejection)

    def test_rejects_explicitly(self) -> None:
        label = make_label("invalid")
        self.assertIsNotNone(label)
        output = transition(reject_command(label))
        self.assertIsNotNone(output.rejection)



if __name__ == "__main__":
    unittest.main()
