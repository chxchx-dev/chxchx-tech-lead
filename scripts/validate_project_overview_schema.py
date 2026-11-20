"""Validate Rust CLI output against its versioned JSON Schemas."""

from __future__ import annotations

import json
import os
import subprocess
import tempfile
from pathlib import Path

from jsonschema import Draft202012Validator


ROOT = Path(__file__).resolve().parents[1]
SCHEMA = ROOT / "schemas" / "project-overview-v1.schema.json"
RESOURCES_SCHEMA = ROOT / "schemas" / "resources-overview-v1.schema.json"
CARGO_MANIFEST = ROOT / "Cargo.toml"


def main() -> int:
    schema = json.loads(SCHEMA.read_text(encoding="utf-8"))
    Draft202012Validator.check_schema(schema)
    resources_schema = json.loads(RESOURCES_SCHEMA.read_text(encoding="utf-8"))
    Draft202012Validator.check_schema(resources_schema)

    with tempfile.TemporaryDirectory(prefix="chxchx-overview-schema-") as home:
        environment = os.environ.copy()
        environment["CHXCHX_TECH_HOME"] = home
        common = [
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
        ]
        for name, args, contract, document in (
            (
                "project overview",
                ["status", str(ROOT), "--json"],
                schema,
                "project-overview-v1.schema.json",
            ),
            (
                "resources overview",
                ["resources", str(ROOT), "--json"],
                resources_schema,
                "resources-overview-v1.schema.json",
            ),
        ):
            result = subprocess.run(
                [*common, *args],
                cwd=ROOT,
                env=environment,
                check=True,
                capture_output=True,
                text=True,
            )
            payload = json.loads(result.stdout)
            errors = sorted(
                Draft202012Validator(contract).iter_errors(payload),
                key=lambda error: list(error.absolute_path),
            )
            if errors:
                details = "\n".join(
                    f"/{'/'.join(map(str, error.absolute_path))}: {error.message}"
                    for error in errors
                )
                raise SystemExit(f"Rust {name} violates its schema:\n{details}")
            print(f"Rust {name} conforms to {document}")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
