from __future__ import annotations

from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))

import golden


def test_cross_language_golden_vectors() -> None:
    golden.test_ticks()
    golden.test_ladders()


def test_public_api_and_schema_helpers() -> None:
    golden.test_public_api_surface()
    golden.test_schema_round_trip()
