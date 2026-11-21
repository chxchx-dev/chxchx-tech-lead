import sys
from pathlib import Path

import pytest

pytest.importorskip("cryptography")
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from sign_manifest import sign  # noqa: E402


def test_the_signature_matches_the_one_the_rust_test_pins():
    # `what_the_ci_script_signs_this_program_accepts` in src/features/release_manifest.rs
    assert sign(b"hola", "05" * 32).startswith("f2d20d2d8e923b3e")


def test_a_key_of_the_wrong_size_is_refused():
    with pytest.raises(ValueError):
        sign(b"x", "05" * 16)
