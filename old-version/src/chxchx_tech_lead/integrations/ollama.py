"""Adapter for a local Ollama: status, model downloads and context variants.

Reads and downloads use Ollama's documented HTTP API on the local server;
creating a model with a larger context uses the official `ollama create`.
Nothing here installs Ollama: `install_instructions` only reports the
official way, so the user runs it with their own privileges.
"""

from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile
import urllib.error
import urllib.request
from collections.abc import Callable
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from ..core.runner import executable as find_executable

DEFAULT_ENDPOINT = "127.0.0.1:11434"
_TIMEOUT_SECONDS = 3
_CREATE_TIMEOUT_SECONDS = 600
_DOWNLOAD_URL = "https://ollama.com/download"
_LINUX_INSTALL = "curl -fsSL https://ollama.com/install.sh | sh"

Opener = Callable[..., Any]
Progress = Callable[[str, int, int], None]


class OllamaError(RuntimeError):
    """Raised when Ollama rejects or cannot complete an operation."""


@dataclass(frozen=True, slots=True)
class InstalledModel:
    name: str
    size_bytes: int


@dataclass(frozen=True, slots=True)
class LoadedModel:
    name: str
    size_bytes: int
    vram_bytes: int


@dataclass(frozen=True, slots=True)
class OllamaStatus:
    installed: bool
    running: bool
    base_url: str
    version: str | None = None
    models: tuple[InstalledModel, ...] = ()
    loaded: tuple[LoadedModel, ...] = ()


def base_url() -> str:
    """Server URL from OLLAMA_HOST, which may omit the scheme, host or port."""
    raw = os.environ.get("OLLAMA_HOST", "").strip() or DEFAULT_ENDPOINT
    scheme, separator, rest = raw.partition("://")
    if not separator:
        scheme, rest = "http", raw
    host, _, port = rest.rstrip("/").partition(":")
    if host in {"", "0.0.0.0", "::"}:
        host = "127.0.0.1"
    return f"{scheme}://{host}:{port or '11434'}"


def install_instructions(platform: str = sys.platform) -> dict[str, str | None]:
    """The official install route; never executed by ChxChx."""
    return {"command": _LINUX_INSTALL if platform.startswith("linux") else None, "url": _DOWNLOAD_URL}


def ollama_status(*, executable: str = "ollama", opener: Opener = urllib.request.urlopen) -> OllamaStatus:
    url = base_url()
    installed = find_executable(executable) is not None
    try:
        version = _get_json(f"{url}/api/version", opener).get("version")
        tags = _get_json(f"{url}/api/tags", opener).get("models", [])
    except (OSError, ValueError):
        return OllamaStatus(installed, False, url)
    models = tuple(
        InstalledModel(str(item["name"]), int(item.get("size") or 0))
        for item in tags if isinstance(item, dict) and item.get("name")
    )
    return OllamaStatus(installed, True, url, str(version) if version else None, models, _loaded_models(url, opener))


def _loaded_models(url: str, opener: Opener) -> tuple[LoadedModel, ...]:
    """Models currently in memory (`/api/ps`); best effort, never fails the status."""
    try:
        running = _get_json(f"{url}/api/ps", opener).get("models", [])
    except (OSError, ValueError):
        return ()
    return tuple(
        LoadedModel(str(item["name"]), int(item.get("size") or 0), int(item.get("size_vram") or 0))
        for item in running if isinstance(item, dict) and item.get("name")
    )


def delete_model(name: str, *, opener: Opener = urllib.request.urlopen) -> None:
    """Remove a downloaded model through Ollama's documented `DELETE /api/delete`."""
    request = urllib.request.Request(
        f"{base_url()}/api/delete",
        data=json.dumps({"model": name}).encode(),
        headers={"Content-Type": "application/json"},
        method="DELETE",
    )
    try:
        with opener(request, timeout=_TIMEOUT_SECONDS * 5):
            return
    except urllib.error.HTTPError as exc:
        raise OllamaError(f"Ollama no pudo borrar {name} (HTTP {exc.code}).") from exc
    except urllib.error.URLError as exc:
        raise OllamaError(f"No pude hablar con Ollama en {base_url()}: {exc.reason}") from exc


def pull_model(model: str, on_progress: Progress, *, opener: Opener = urllib.request.urlopen) -> None:
    """Download `model`, calling `on_progress(status, completed, total)` per update.

    Raising from the callback stops the download: the stream is closed.
    """
    request = urllib.request.Request(
        f"{base_url()}/api/pull",
        data=json.dumps({"model": model, "stream": True}).encode(),
        headers={"Content-Type": "application/json"},
    )
    try:
        with opener(request, timeout=_TIMEOUT_SECONDS * 20) as response:
            for line in response:
                if not line.strip():
                    continue
                event = json.loads(line)
                if event.get("error"):
                    raise OllamaError(str(event["error"]))
                on_progress(str(event.get("status", "")), int(event.get("completed") or 0), int(event.get("total") or 0))
    except urllib.error.URLError as exc:
        raise OllamaError(f"No pude hablar con Ollama en {base_url()}: {exc.reason}") from exc
    except json.JSONDecodeError as exc:
        raise OllamaError("Ollama devolvió una respuesta que no es JSON") from exc


def create_context_variant(
    base: str, context: int, *, executable: str = "ollama", run: Callable[..., Any] = subprocess.run
) -> str:
    """Create `<base>-ctx<N>k` with a larger context and return its name.

    Ollama's default 4096-token window breaks OpenCode's tool calls and the
    OpenAI-compatible endpoint cannot raise it per request, so the window is
    baked into a model that shares the base weights.
    """
    name = f"{base}-ctx{context // 1024}k"
    with tempfile.TemporaryDirectory() as folder:
        modelfile = Path(folder) / "Modelfile"
        modelfile.write_text(f"FROM {base}\nPARAMETER num_ctx {context}\n", encoding="utf-8")
        try:
            result = run([find_executable(executable) or executable, "create", name, "-f", str(modelfile)], capture_output=True, text=True,
                         encoding="utf-8", errors="replace", timeout=_CREATE_TIMEOUT_SECONDS, check=False)
        except (OSError, subprocess.TimeoutExpired) as exc:
            raise OllamaError(f"No pude ejecutar `ollama create`: {exc}") from exc
    if result.returncode != 0:
        raise OllamaError((result.stderr or result.stdout or "ollama create falló").strip()[-400:])
    return name


def _get_json(url: str, opener: Opener) -> dict[str, Any]:
    with opener(url, timeout=_TIMEOUT_SECONDS) as response:
        data = json.loads(response.read())
    if not isinstance(data, dict):
        raise ValueError("respuesta inesperada")
    return data
