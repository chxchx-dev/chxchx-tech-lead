"""Free an agent's context window automatically, without the person noticing.

Agents get slower and costlier as their context fills. When a visual-chat turn ends with the context
at 90% or more, the agent writes a hand-off of the conversation (in that same session, which still
knows everything), the next message starts a fresh session that receives that hand-off together with
the project context files, and the chat carries on. The conversation shown in Studio never changes.
"""

from __future__ import annotations

import json
import os
import tempfile
from pathlib import Path
from typing import Any

from ..integrations.explicit_memory import contains_secret

THRESHOLD = 0.90
_HANDOFF_DIR = Path(".ai") / "chat-handoffs"
_MAX_HANDOFF_CHARS = 24_000

HANDOFF_PROMPT = """\
Vamos a liberar tu ventana de contexto: esta conversación seguirá en una sesión nueva que solo conocerá lo que \
escribas ahora. No uses herramientas ni modifiques archivos. Redacta un traspaso completo y fiel, en Markdown, \
con estas secciones:

## Objetivo
Qué quiere lograr la persona y en qué contexto.

## Decisiones y preferencias
Decisiones tomadas, estilo y restricciones, lo que pidió o rechazó (con sus palabras cuando importen).

## Lo hecho
Archivos creados o modificados (ruta y qué cambió), comandos relevantes con su resultado y pruebas ejecutadas.

## Estado actual
Qué funciona, qué falla y las hipótesis vigentes.

## Pendiente
Lo que falta y el siguiente paso.

## Datos que no hay que volver a buscar
Rutas, nombres, identificadores y mensajes de error exactos.

Si al empezar esta conversación recibiste un traspaso anterior, incorpora lo que siga vigente: un traspaso sobre otro \
no debe perder nada. Máximo 1500 palabras. No incluyas secretos ni tokens. Responde solo con el traspaso."""


def context_ratio(tokens: int | None, window: int | None) -> float | None:
    if not tokens or not window or window <= 0:
        return None
    return tokens / window


def needs_release(tokens: int | None, window: int | None) -> bool:
    ratio = context_ratio(tokens, window)
    return ratio is not None and ratio >= THRESHOLD


# -- the hand-off file ------------------------------------------------------------------------------------------------------
def _file(project_root: Path, provider: str, session_id: str) -> Path:
    safe = "".join(char for char in f"{provider.lower()}-{session_id}" if char.isalnum() or char in "-_.")
    return project_root / _HANDOFF_DIR / f"{safe}.md"


def _without_secrets(text: str) -> str:
    """The agent was told not to write secrets; a line that looks like one is dropped anyway."""
    return "\n".join("(línea omitida: posible secreto)" if contains_secret(line) else line for line in text.splitlines())


def save_handoff(project_root: Path, provider: str, session_id: str, text: str) -> Path:
    target = _file(project_root, provider, session_id)
    target.parent.mkdir(parents=True, exist_ok=True)
    content = _without_secrets(text.strip())[:_MAX_HANDOFF_CHARS] + "\n"
    descriptor, temporary = tempfile.mkstemp(prefix=f".{target.name}.", dir=target.parent)
    try:
        with os.fdopen(descriptor, "w", encoding="utf-8", newline="\n") as stream:
            stream.write(content)
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(temporary, target)  # closing or crashing mid-write never leaves a truncated hand-off
    finally:
        if os.path.exists(temporary):
            os.unlink(temporary)
    return target


def load_handoff(project_root: Path, provider: str, session_id: str | None) -> str:
    if not session_id:
        return ""
    try:
        return _file(project_root, provider, session_id).read_text(encoding="utf-8").strip()
    except (OSError, UnicodeError):
        return ""


def consume_handoff(project_root: Path, provider: str, session_id: str | None) -> None:
    """The new session has it now: the file is no longer needed."""
    if session_id:
        try:
            _file(project_root, provider, session_id).unlink()
        except OSError:
            pass


def handoff_block(text: str) -> str:
    """The hand-off as context for the first message of the new session (a block memory capture ignores)."""
    return (
        "\n\nEsta conversación continúa de una sesión anterior cuyo contexto se liberó automáticamente para ahorrar "
        "consumo. Retómala desde el traspaso de abajo como si nada hubiera cambiado: no menciones el traspaso ni que "
        "el contexto se liberó, no pidas confirmación ni repitas lo ya hecho. Si la persona pregunta por algo que "
        "está en el traspaso, responde con naturalidad, como si lo recordaras de la conversación.\n"
        f'<archivo-de-contexto ruta="traspaso de la conversación">\n{text}\n</archivo-de-contexto>'
    )


# -- the context window of models that do not report it --------------------------------------------------------------------
def _read_json(path: Path) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError):
        return None


def _limit(data: Any, provider: str, model: str) -> int | None:
    try:
        value = data[provider]["models"][model]["limit"]["context"]
    except (KeyError, TypeError):
        return None
    return value if isinstance(value, int) and value > 0 else None


def opencode_model(configured: str | None = None) -> str | None:
    """`provider/model` OpenCode will use: the one asked for, or the default of its configuration."""
    if configured and "/" in configured:
        return configured
    config_home = Path(os.environ.get("XDG_CONFIG_HOME", Path.home() / ".config")).expanduser()
    for name in ("opencode.json", "opencode.jsonc"):
        config = _read_json(config_home / "opencode" / name)
        if isinstance(config, dict) and isinstance(config.get("model"), str) and "/" in config["model"]:
            return config["model"]
    return None


def opencode_window(model: str | None) -> int | None:
    """Context window of an OpenCode model: from the person's provider settings, else OpenCode's own models cache."""
    model = opencode_model(model)
    if not model:
        return None
    provider, _, name = model.partition("/")
    config_home = Path(os.environ.get("XDG_CONFIG_HOME", Path.home() / ".config")).expanduser()
    for file in ("opencode.json", "opencode.jsonc"):
        config = _read_json(config_home / "opencode" / file)
        found = _limit(config.get("provider") if isinstance(config, dict) else None, provider, name)
        if found:
            return found
    cache_home = Path(os.environ.get("XDG_CACHE_HOME", Path.home() / ".cache")).expanduser()
    return _limit(_read_json(cache_home / "opencode" / "models.json"), provider, name)


class UsageWatch:
    """Follows the context usage a turn reports, completing the window for models that do not report it."""

    def __init__(self, provider: str, model: str | None) -> None:
        self._provider, self._model = provider, model
        self.tokens: int | None = None
        self.window: int | None = None

    @property
    def usage(self) -> tuple[int | None, int | None]:
        return self.tokens, self.window

    def observe(self, event: dict[str, Any], model_seen: str | None = None) -> dict[str, Any]:
        if event.get("type") != "limits":
            return event
        window = event.get("context_window")
        if window is None and self._provider == "OpenCode":
            window = opencode_window(self._model or model_seen)
            if window:
                event = {**event, "context_window": window}
        self.tokens = event.get("context_tokens") or self.tokens
        self.window = window if isinstance(window, int) else self.window
        return event
