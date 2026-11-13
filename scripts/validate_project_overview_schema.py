"""Validate the Rust CLI's actual project overview output against its JSON Schema."""

from __future__ import annotations

import json
import os
import subprocess
import tempfile
from pathlib import Path

from jsonschema import Draft202012Validator


ROOT = Path(__file__).resolve().parents[1]
SCHEMA = ROOT / "schemas" / "project-overview-v1.schema.json"
CARGO_MANIFEST = ROOT / "migration" / "rust" / "Cargo.toml"


def main() -> int:
    schema = json.loads(SCHEMA.read_text(encoding="utf-8"))
    Draft202012Validator.check_schema(schema)

    with tempfile.TemporaryDirectory(prefix="chxchx-overview-schema-") as home:
        environment = os.environ.copy()
        environment["CHXCHX_TECH_HOME"] = home
        result = subprocess.run(
            [
                "cargo",
                "run",
                "--quiet",
                "--manifest-path",
                str(CARGO_MANIFEST),
                "--locked",
                "--no-default-features",
                "--features",
                "cli",
                "--bin",
                "chxchx-tech",
                "--",
                "status",
                str(ROOT),
                "--json",
            ],
            cwd=ROOT,
            env=environment,
            check=True,
            capture_output=True,
            text=True,
        )
        overview = json.loads(result.stdout)
        errors = sorted(
            Draft202012Validator(schema).iter_errors(overview),
            key=lambda error: list(error.absolute_path),
        )
        if errors:
            details = "\n".join(
                f"/{'/'.join(map(str, error.absolute_path))}: {error.message}"
                for error in errors
            )
            raise SystemExit(f"Rust project overview violates its schema:\n{details}")

    print("Rust project overview conforms to project-overview-v1.schema.json")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
