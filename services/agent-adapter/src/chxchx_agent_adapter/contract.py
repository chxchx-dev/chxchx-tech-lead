"""The versioned contract between the Rust product and this service.

Rust owns trust, configuration, access policy, context and the visible transcript; it decides
exactly which process to run and how. This service only drives that process: it speaks the
provider's live protocol, turns what it prints into `chxchx.agent-event` v1 events and relays the
person's answers. Nothing here reads project files or settings of ChxChx.

Run:  `chxchx-agent-adapter run`
  stdin  line 1   one request (below), a single JSON object
  stdin  line 2…  `{"type": "answer", "request_id": str, ...}` for a permission or a question,
                  `cancel` to stop the turn. The end of the input also stops it.
  stdout          one `chxchx.agent-event` v1 object per line (see the event types in the Rust
                  `chat_model`), ending with exactly one `adapter_done`:
                  `{"type": "adapter_done", "exit_code": int, "cancelled": bool, "session_id": str | null}`
  exit code       the provider's own exit code (0 when the turn ended or was stopped by the host),
                  2 for a request that breaks this contract.

Request, `schema` = `chxchx.agent-adapter-request`, `schema_version` = 1:
  provider        "Codex" | "OpenCode"
  argv            the process to run: `codex app-server …` or `opencode serve …`
  cwd             working directory of that process (an existing folder)
  env             extra environment for it (merged over this service's own)
  prompt          the message to send
  session_id      the provider session to resume, or null
  model, effort   optional strings
  images          paths of images to attach
  codex           Codex only: {"sandbox", "approvalPolicy", "sandboxPolicy"} as Rust computed them
  opencode        OpenCode only: {"agent": "build" | "plan", "major": 1 | 2 | null}
"""

from __future__ import annotations

import json
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

REQUEST_SCHEMA = "chxchx.agent-adapter-request"
EVENT_SCHEMA = "chxchx.agent-event"
CONTRACT_VERSION = 1
PROVIDERS = ("Codex", "OpenCode")
# What the first words of `argv` must be: the service never runs anything else.
_SUBCOMMAND = {"Codex": "app-server", "OpenCode": "serve"}
_PROGRAM = {"Codex": "codex", "OpenCode": "opencode"}
_MAX_REQUEST_BYTES = 8 * 1024 * 1024


class ContractError(ValueError):
    """The request does not follow the contract."""


@dataclass(frozen=True, slots=True)
class Request:
    provider: str
    argv: list[str]
    cwd: Path
    env: dict[str, str]
    prompt: str
    session_id: str | None = None
    model: str | None = None
    effort: str | None = None
    images: list[Path] = field(default_factory=list)
    codex: dict[str, Any] = field(default_factory=dict)
    opencode_agent: str = "build"
    opencode_major: int | None = None


def _text(value: Any, name: str, *, required: bool = False) -> str | None:
    if value is None or value == "":
        if required:
            raise ContractError(f"Falta `{name}`")
        return None
    if not isinstance(value, str):
        raise ContractError(f"`{name}` debe ser texto")
    return value


def parse_request(line: str) -> Request:
    if len(line.encode("utf-8", errors="replace")) > _MAX_REQUEST_BYTES:
        raise ContractError("La solicitud es demasiado grande")
    try:
        data = json.loads(line)
    except json.JSONDecodeError as error:
        raise ContractError(f"La solicitud no es JSON: {error}") from error
    if not isinstance(data, dict) or data.get("schema") != REQUEST_SCHEMA:
        raise ContractError("La solicitud no es de este servicio")
    if data.get("schema_version") != CONTRACT_VERSION:
        raise ContractError(f"Versión de contrato no soportada: {data.get('schema_version')!r}")
    provider = data.get("provider")
    if provider not in PROVIDERS:
        raise ContractError(f"Proveedor no soportado: {provider!r}")
    argv = data.get("argv")
    if not isinstance(argv, list) or len(argv) < 2 or not all(isinstance(item, str) and item for item in argv):
        raise ContractError("`argv` debe ser una lista de textos con el programa y su subcomando")
    program = Path(argv[0].replace("\\", "/")).name.casefold()
    for suffix in (".exe", ".cmd", ".bat"):
        program = program.removesuffix(suffix)
    if program != _PROGRAM[provider] or argv[1] != _SUBCOMMAND[provider]:
        raise ContractError(f"Para {provider} solo se ejecuta `{_PROGRAM[provider]} {_SUBCOMMAND[provider]}`")
    cwd = Path(_text(data.get("cwd"), "cwd", required=True) or "")
    if not cwd.is_dir():
        raise ContractError(f"No existe la carpeta de trabajo: {cwd}")
    raw_env = data.get("env", {})
    if not isinstance(raw_env, dict) or not all(isinstance(k, str) and isinstance(v, str) for k, v in raw_env.items()):
        raise ContractError("`env` debe ser un objeto de textos")
    prompt = data.get("prompt")
    if not isinstance(prompt, str) or not prompt.strip():
        raise ContractError("Falta `prompt`")
    raw_images = data.get("images", [])
    if not isinstance(raw_images, list) or not all(isinstance(item, str) for item in raw_images):
        raise ContractError("`images` debe ser una lista de rutas")
    images = [Path(item) for item in raw_images]
    for image in images:
        if not image.is_file():
            raise ContractError(f"No existe la imagen: {image}")
    codex: dict[str, Any] = {}
    agent, major = "build", None
    if provider == "Codex":
        codex = data.get("codex") if isinstance(data.get("codex"), dict) else {}
        if not all(key in codex for key in ("sandbox", "approvalPolicy", "sandboxPolicy")):
            raise ContractError("`codex` necesita sandbox, approvalPolicy y sandboxPolicy")
    else:
        options = data.get("opencode") if isinstance(data.get("opencode"), dict) else {}
        agent = options.get("agent", "build")
        if agent not in ("build", "plan"):
            raise ContractError("`opencode.agent` debe ser build o plan")
        major = options.get("major")
        if major is not None and not (isinstance(major, int) and not isinstance(major, bool) and major > 0):
            raise ContractError("`opencode.major` debe ser un entero positivo o null")
    return Request(provider, argv, cwd, dict(raw_env), prompt, _text(data.get("session_id"), "session_id"),
                   _text(data.get("model"), "model"), _text(data.get("effort"), "effort"), images, codex, agent, major)
