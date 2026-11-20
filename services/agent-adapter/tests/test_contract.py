"""The request is validated before anything runs: the service never starts what the contract does not allow."""

from __future__ import annotations

import json
from pathlib import Path

import pytest

from chxchx_agent_adapter.contract import ContractError, parse_request


def _line(root: Path, **changes) -> str:
    request = {"schema": "chxchx.agent-adapter-request", "schema_version": 1, "provider": "Codex",
               "argv": ["codex", "app-server"], "cwd": str(root), "env": {"A": "b"}, "prompt": "hola",
               "codex": {"sandbox": "read-only", "approvalPolicy": "never", "sandboxPolicy": {}}}
    request.update(changes)
    return json.dumps({k: v for k, v in request.items() if v is not None})


def test_a_valid_codex_request_is_read(tmp_path: Path) -> None:
    request = parse_request(_line(tmp_path, session_id="s1", model="m", effort="high"))

    assert (request.provider, request.argv, request.cwd, request.env) == ("Codex", ["codex", "app-server"], tmp_path, {"A": "b"})
    assert (request.session_id, request.model, request.effort) == ("s1", "m", "high")


def test_a_valid_opencode_request_defaults_to_the_build_agent(tmp_path: Path) -> None:
    request = parse_request(_line(tmp_path, provider="OpenCode", argv=["/opt/opencode.exe", "serve", "--port", "0"], codex=None))

    assert request.provider == "OpenCode" and request.opencode_agent == "build" and request.opencode_major is None


@pytest.mark.parametrize("changes,message", [
    ({"schema": "otra"}, "no es de este servicio"),
    ({"schema_version": 2}, "Versión de contrato"),
    ({"provider": "Claude"}, "Proveedor"),
    ({"argv": ["codex"]}, "argv"),
    ({"argv": ["bash", "app-server"]}, "solo se ejecuta"),
    ({"argv": ["codex", "exec"]}, "solo se ejecuta"),
    ({"argv": ["opencode", "serve"]}, "solo se ejecuta"),   # an OpenCode command for a Codex request
    ({"cwd": "/no/existe/nunca"}, "carpeta de trabajo"),
    ({"env": {"A": 1}}, "env"),
    ({"prompt": "  "}, "prompt"),
    ({"images": ["/no/existe.png"]}, "imagen"),
    ({"codex": {"sandbox": "read-only"}}, "codex"),
])
def test_requests_that_break_the_contract_are_refused(tmp_path: Path, changes, message) -> None:
    with pytest.raises(ContractError, match=message):
        parse_request(_line(tmp_path, **changes))


def test_opencode_options_are_checked(tmp_path: Path) -> None:
    base = {"provider": "OpenCode", "argv": ["opencode", "serve"], "codex": None}
    parse_request(_line(tmp_path, opencode={"agent": "plan", "major": 2}, **base))
    for bad in ({"agent": "root"}, {"major": 0}, {"major": True}):
        with pytest.raises(ContractError):
            parse_request(_line(tmp_path, opencode=bad, **base))


def test_garbage_is_not_a_request() -> None:
    for text in ("", "no es json", "[]", "null"):
        with pytest.raises(ContractError):
            parse_request(text)
