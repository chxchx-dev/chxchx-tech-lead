"""Helpers for the live tests, which talk to the real opencode and ollama.

Run them with `uv run pytest -m live -v`. Set CHXCHX_LIVE_CAPTURE=<dir> to also
save the raw `opencode run --format json` output, to refresh test fixtures.
"""

from __future__ import annotations

import os
import shutil
import subprocess
from pathlib import Path
from types import SimpleNamespace

import pytest

PROMPT_TIMEOUT_SECONDS = int(os.environ.get("CHXCHX_LIVE_TIMEOUT", "180"))


def pytest_collection_modifyitems(items):
    for item in items:
        if "tests/live" in item.nodeid.replace("\\", "/"):
            item.add_marker(pytest.mark.live)


@pytest.fixture
def opencode_binary() -> str:
    binary = shutil.which("opencode")
    if binary is None:
        pytest.skip("opencode no está instalado (npm install -g opencode-ai) y autenticado con un proveedor")
    return binary


@pytest.fixture
def ollama_binary() -> str:
    binary = shutil.which("ollama")
    if binary is None:
        pytest.skip("ollama no está instalado (https://ollama.com/download)")
    return binary


@pytest.fixture
def project(tmp_path: Path) -> Path:
    root = tmp_path / "proyecto"
    root.mkdir()
    return root


@pytest.fixture
def run_opencode(opencode_binary: str):
    """Run one `opencode run --format json` turn with the prompt on stdin, like ChxChx does."""

    def run(project: Path, prompt: str, *extra: str, env: dict[str, str] | None = None) -> subprocess.CompletedProcess[str]:
        model = os.environ.get("CHXCHX_LIVE_OPENCODE_MODEL")
        argv = [opencode_binary, "run", "--format", "json", *(("--model", model) if model else ()), *extra]
        result = subprocess.run(argv, input=prompt, cwd=project, capture_output=True, text=True, encoding="utf-8",
                                timeout=PROMPT_TIMEOUT_SECONDS, env={**os.environ, **(env or {})}, check=False)
        capture = os.environ.get("CHXCHX_LIVE_CAPTURE")
        if capture:
            folder = Path(capture)
            folder.mkdir(parents=True, exist_ok=True)
            (folder / f"opencode-{abs(hash((prompt, extra))) % 10**8}.jsonl").write_text(result.stdout, encoding="utf-8")
        return result

    return run


def service_for(root: Path, command: list[str]):
    agent = SimpleNamespace(id="live", command=command, cwd=".")
    inspection = SimpleNamespace(trusted=True, config=SimpleNamespace(agents=[agent]))
    return SimpleNamespace(inspect=lambda: inspection, project=SimpleNamespace(root=root))
