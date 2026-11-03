"""Fake `ollama` and `opencode` executables that behave like the real CLIs' contracts.

They are real processes found through PATH, so argv, stdin, environment,
exit codes and JSON-lines output are exercised exactly as in production.
Each writes what it received to the JSON-lines file named by FAKE_CLI_LOG.
"""

from __future__ import annotations

import os
import stat
import sys
from pathlib import Path

OLLAMA = r'''
import json, os, sys, urllib.request
args = sys.argv[1:]
log = os.environ.get("FAKE_CLI_LOG")
if log:
    with open(log, "a", encoding="utf-8") as stream:
        stream.write(json.dumps({"tool": "ollama", "argv": args}) + "\n")
if args[:1] == ["create"]:
    name, modelfile = args[1], args[args.index("-f") + 1]
    text = open(modelfile, encoding="utf-8").read()
    if os.environ.get("FAKE_OLLAMA_CREATE_FAILS"):
        print("Error: pull model manifest: file does not exist", file=sys.stderr)
        sys.exit(1)
    base = text.splitlines()[0].removeprefix("FROM ").strip()
    host = os.environ.get("OLLAMA_HOST", "127.0.0.1:11434")
    request = urllib.request.Request(f"http://{host}/api/create", method="POST",
        data=json.dumps({"model": name, "from": base, "modelfile": text}).encode(),
        headers={"Content-Type": "application/json"})
    urllib.request.urlopen(request, timeout=5).read()
    print("success")
'''

OPENCODE = r'''
import json, os, sys, time
args = sys.argv[1:]
stdin_text = "" if sys.stdin.isatty() else sys.stdin.buffer.read().decode("utf-8")
log = os.environ.get("FAKE_CLI_LOG")
if log:
    with open(log, "a", encoding="utf-8") as stream:
        stream.write(json.dumps({"tool": "opencode", "argv": args, "stdin": stdin_text,
                                 "permission": os.environ.get("OPENCODE_PERMISSION"), "cwd": os.getcwd()}) + "\n")
now = int(time.time() * 1000)
if args[:2] == ["session", "list"]:
    print(json.dumps([{"id": "ses_fake1", "directory": os.getcwd(), "time": {"updated": now}}]))
elif args[:2] == ["session", "delete"]:
    print("deleted")
elif args[:1] == ["export"]:
    messages = [{"info": {"role": "user"}, "parts": [{"type": "text", "text": "Arregla la memoria por chat"}]},
                {"info": {"role": "assistant"}, "parts": [{"type": "text", "text": "Listo: quedó arreglado."}]}]
    print(json.dumps({"info": {"directory": os.getcwd()}, "messages": messages}))
elif args[:1] == ["models"]:
    print("anthropic/claude-sonnet-5-5\nollama/qwen3-coder:30b-ctx32k")
elif args[:1] == ["run"]:
    session = args[args.index("--session") + 1] if "--session" in args else "ses_fake1"
    def emit(kind, part=None, **extra):
        print(json.dumps({"type": kind, "timestamp": now, "sessionID": session, **({"part": part} if part else {}), **extra}), flush=True)
    emit("step_start", {"type": "step-start"})
    if "FALLA" in stdin_text:
        emit("error", error={"name": "APIError", "data": {"message": "sin cuota"}})
        sys.exit(1)
    emit("tool_use", {"type": "tool", "callID": "c1", "tool": "bash",
                      "state": {"status": "completed", "input": {"command": "ls"}, "output": "a.py\n"}})
    emit("text", {"type": "text", "text": "eco: " + stdin_text.strip().splitlines()[-1]})
    emit("step_finish", {"type": "step-finish", "reason": "stop", "cost": 0.002,
                         "tokens": {"input": 100, "output": 7, "reasoning": 0, "cache": {"read": 50, "write": 0}}})
'''


def install(directory: Path, name: str, source: str) -> Path:
    """Write an executable `name` into `directory` (plus a .cmd shim on Windows)."""
    directory.mkdir(parents=True, exist_ok=True)
    script = directory / f"{name}-fake.py"
    script.write_text(source, encoding="utf-8")
    if os.name == "nt":
        shim = directory / f"{name}.cmd"
        shim.write_text(f'@echo off\r\n"{sys.executable}" "{script}" %*\r\n', encoding="utf-8")
        return shim
    launcher = directory / name
    launcher.write_text(f"#!{sys.executable}\nimport runpy, sys\nsys.argv[0] = {str(script)!r}\nrunpy.run_path({str(script)!r}, run_name='__main__')\n",
                        encoding="utf-8")
    launcher.chmod(launcher.stat().st_mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH)
    return launcher
