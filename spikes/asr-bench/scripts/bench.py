#!/usr/bin/env python3
"""
Fallback harness using sherpa-onnx pip if the Rust crate does not build.

  python3 -m venv .venv && source .venv/bin/activate
  pip install sherpa-onnx soundfile
  python3 spikes/asr-bench/scripts/bench.py --help
"""

from __future__ import annotations

import argparse
import csv
import sys
import time
from pathlib import Path

SPIKE_ROOT = Path(__file__).resolve().parents[1]
DEFAULT_MODELS = SPIKE_ROOT / "models"
DEFAULT_FIXTURES = SPIKE_ROOT / "fixtures"


def main() -> int:
    try:
        import sherpa_onnx  # noqa: F401
    except ImportError:
        print(
            "sherpa-onnx Python package not installed. "
            "pip install sherpa-onnx soundfile",
            file=sys.stderr,
        )
        return 1

    parser = argparse.ArgumentParser(description="ASR bench (Python fallback)")
    parser.add_argument("--models-dir", type=Path, default=DEFAULT_MODELS)
    parser.add_argument("--fixtures-dir", type=Path, default=DEFAULT_FIXTURES)
    parser.add_argument("--model", action="append", default=[])
    args = parser.parse_args()

    print(
        "Python fallback: implement recognizer wiring mirroring models.toml "
        "or use spike-asr-bench (Rust).",
        file=sys.stderr,
    )
    return 2


if __name__ == "__main__":
    raise SystemExit(main())
