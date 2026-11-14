"""Point a project's `opencode.json` at models served by a local Ollama.

The merge is additive and idempotent: only `provider.ollama` and, when asked,
`model` change; every other key the user wrote is kept. The file is written
atomically after a backup, and a dry run returns the exact text that would be
written.
"""

from __future__ import annotations

import json
import os
import tempfile
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from ..core.backup import backup_file

CONFIG_NAME = "opencode.json"
BACKUP_AREA = "backups-opencode"
SCHEMA_URL = "https://opencode.ai/config.json"
PROVIDER_ID = "ollama"
_OUTPUT_TOKENS = 8192
_OWNED_PROVIDER_KEYS = {"npm", "name", "options", "models"}


class OpenCodeConfigError(ValueError):
    """Raised when the existing config cannot be merged without losing data."""


@dataclass(frozen=True, slots=True)
class ConfigChange:
    path: Path
    before: str
    after: str
    model_ref: str

    @property
    def changed(self) -> bool:
        return self.before != self.after


def plan_ollama_model(
    root: Path, tag: str, *, label: str, context: int, endpoint: str, set_default: bool
) -> ConfigChange:
    path, before, data = _load(root)
    data.setdefault("$schema", SCHEMA_URL)
    provider = _object(data, "provider")
    entry = _object(provider, PROVIDER_ID)
    entry.setdefault("npm", "@ai-sdk/openai-compatible")
    entry.setdefault("name", "Ollama (local)")
    options = _object(entry, "options")
    options.setdefault("baseURL", f"{endpoint.rstrip('/')}/v1")
    model = _object(_object(entry, "models"), tag)
    model["name"] = label
    model["limit"] = {"context": context, "output": min(_OUTPUT_TOKENS, context // 4)}
    ref = f"{PROVIDER_ID}/{tag}"
    if set_default:
        data["model"] = ref
    return ConfigChange(path, before, json.dumps(data, indent=2, ensure_ascii=False) + "\n", ref)


def plan_remove_ollama_models(root: Path, tag: str) -> ConfigChange:
    """Drop `tag` and its `-ctx` variants from the config, leaving everything else untouched.

    A default `model` pointing at a removed entry is cleared, because OpenCode
    would fail on a model that no longer exists. An Ollama provider left with
    no models is removed too, but only if it holds nothing but what we wrote.
    """
    path, before, data = _load(root)
    provider = data.get("provider") if isinstance(data.get("provider"), dict) else {}
    entry = provider.get(PROVIDER_ID) if isinstance(provider.get(PROVIDER_ID), dict) else None
    models = entry.get("models") if entry is not None and isinstance(entry.get("models"), dict) else {}
    removed = [name for name in models if name == tag or name.startswith(f"{tag}-ctx")]
    for name in removed:
        del models[name]
    if data.get("model") in {f"{PROVIDER_ID}/{name}" for name in removed}:
        del data["model"]
    if entry is not None and removed and not models and set(entry) <= _OWNED_PROVIDER_KEYS:
        del provider[PROVIDER_ID]
        if not provider:
            del data["provider"]
    after = json.dumps(data, indent=2, ensure_ascii=False) + "\n" if removed else before
    return ConfigChange(path, before, after, f"{PROVIDER_ID}/{tag}")


def apply_change(root: Path, change: ConfigChange) -> Path | None:
    """Write the planned config; returns the backup of the previous file, if any."""
    if not change.changed:
        return None
    backup = backup_file(root, CONFIG_NAME, BACKUP_AREA)
    descriptor, temporary = tempfile.mkstemp(dir=root, prefix=".opencode-", suffix=".tmp")
    try:
        with os.fdopen(descriptor, "w", encoding="utf-8") as stream:
            stream.write(change.after)
        os.replace(temporary, change.path)
    except BaseException:
        Path(temporary).unlink(missing_ok=True)
        raise
    return backup


def configured_ollama_models(root: Path) -> list[str]:
    try:
        data = json.loads((root / CONFIG_NAME).read_text(encoding="utf-8"))
        models = data["provider"][PROVIDER_ID]["models"]
    except (OSError, ValueError, KeyError, TypeError):
        return []
    return sorted(models) if isinstance(models, dict) else []


def _load(root: Path) -> tuple[Path, str, dict[str, Any]]:
    path = root / CONFIG_NAME
    if not path.exists() and (root / "opencode.jsonc").exists():
        raise OpenCodeConfigError("El proyecto usa opencode.jsonc (con comentarios); edítalo a mano para no perderlos.")
    before = path.read_text(encoding="utf-8") if path.exists() else ""
    try:
        data = json.loads(before) if before.strip() else {}
    except json.JSONDecodeError as exc:
        raise OpenCodeConfigError(f"{CONFIG_NAME} no es JSON válido ({exc.msg}, línea {exc.lineno}); no lo modifico.") from exc
    if not isinstance(data, dict):
        raise OpenCodeConfigError(f"{CONFIG_NAME} debe contener un objeto JSON.")
    return path, before, data


def _object(parent: dict[str, Any], key: str) -> dict[str, Any]:
    value = parent.setdefault(key, {})
    if not isinstance(value, dict):
        raise OpenCodeConfigError(f"`{key}` en {CONFIG_NAME} debe ser un objeto.")
    return value
