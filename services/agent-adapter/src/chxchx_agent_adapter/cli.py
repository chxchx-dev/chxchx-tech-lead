"""`chxchx-agent-adapter`: the command line of the service (see `contract`)."""

from __future__ import annotations

import json
import sys
from typing import IO

from . import VERSION
from .contract import CONTRACT_VERSION, PROVIDERS, ContractError, parse_request
from .runner import json_line_emitter, run_turn

_BAD_REQUEST = 2


def describe() -> dict[str, object]:
    return {"schema": "chxchx.agent-adapter", "contract_version": CONTRACT_VERSION, "version": VERSION,
            "providers": list(PROVIDERS)}


def main(argv: list[str] | None = None, stdin: IO[str] | None = None, stdout: IO[str] | None = None) -> int:
    arguments = sys.argv[1:] if argv is None else argv
    source, sink = stdin or sys.stdin, stdout or sys.stdout
    for stream in (source, sink):
        reconfigure = getattr(stream, "reconfigure", None)
        if reconfigure is not None:
            reconfigure(encoding="utf-8", errors="replace")
    command = arguments[0] if arguments else ""
    if command in {"--version", "version", "contract"}:
        sink.write(json.dumps(describe()) + "\n")
        sink.flush()
        return 0
    if command != "run":
        sys.stderr.write("Uso: chxchx-agent-adapter run | contract | --version\n")
        return _BAD_REQUEST
    emit = json_line_emitter(sink)
    try:
        request = parse_request(source.readline())
    except ContractError as error:
        emit({"type": "error", "message": str(error)})
        emit({"type": "adapter_done", "exit_code": _BAD_REQUEST, "cancelled": False, "session_id": None})
        return _BAD_REQUEST
    return run_turn(request, emit, source)  # the provider's own exit code
