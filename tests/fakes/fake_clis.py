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
if args[:1] == ["serve"]:
    import runpy
    runpy.run_path(os.path.join(os.path.dirname(os.path.realpath(sys.argv[0])), "opencode-serve-fake.py"), run_name="__main__")
    sys.exit(0)
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


# A `opencode serve` (HTTP + server-sent events) that asks the host before it acts, like the real one.
OPENCODE_SERVE = r'''
import base64, http.server, json, os, queue, sys, threading, time
log = os.environ.get("FAKE_CLI_LOG")
password = os.environ.get("OPENCODE_SERVER_PASSWORD", "")
def note(**fields):
    if log:
        with open(log, "a", encoding="utf-8") as stream:
            stream.write(json.dumps({"tool": "opencode-serve", **fields}) + "\n")
note(argv=sys.argv[1:], permission=os.environ.get("OPENCODE_PERMISSION"), has_password=bool(password))
listeners, lock = [], threading.Lock()
replies = {}
def publish(kind, **props):
    with lock:
        for q in listeners:
            q.put({"type": kind, "properties": props})
def wait_reply(request_id):
    for _ in range(300):
        if request_id in replies:
            return replies[request_id]
        time.sleep(0.05)
    return None
def scenario(session, text):
    base = {"sessionID": session}
    publish("session.status", **base, status={"type": "busy"})
    publish("message.updated", **base, info={"id": "msg_u", "role": "user"})
    publish("message.updated", **base, info={"id": "msg_a", "role": "assistant"})
    reply = None
    if "PERMISO" in text:
        publish("permission.asked", id="per_1", sessionID=session, permission="bash", patterns=["python3 -V"],
                metadata={"command": "python3 -V"}, always=["python3 *"], tool={"messageID": "msg_a", "callID": "call_1"})
        reply = wait_reply("per_1")
        part = {"id": "prt_t", "messageID": "msg_a", "sessionID": session, "type": "tool", "tool": "bash", "callID": "call_1",
                "state": {"status": "completed" if reply == "once" else "error", "input": {"command": "python3 -V"}, "output": "Python 3.11",
                          "error": "The user rejected permission to use this specific tool call."}}
        publish("message.part.updated", **base, part=part)
        answer = "decision: " + json.dumps(reply)
    elif "PREGUNTA" in text:
        publish("question.asked", id="que_1", sessionID=session, questions=[
            {"question": "Color?", "header": "Color", "options": [{"label": "Rojo", "description": ""}, {"label": "Azul", "description": ""}], "multiple": False}])
        reply = wait_reply("que_1")
        answer = "respuesta: " + json.dumps(reply, ensure_ascii=False)
    else:
        answer = "eco: " + text.strip().splitlines()[-1]
    publish("message.part.updated", **base, part={"id": "prt_x", "messageID": "msg_a", "sessionID": session, "type": "text", "text": answer, "time": {"start": 1, "end": 2}})
    publish("message.part.updated", **base, part={"id": "prt_s", "messageID": "msg_a", "sessionID": session, "type": "step-finish",
            "cost": 0.001, "tokens": {"input": 100, "output": 7, "reasoning": 0, "cache": {"read": 50, "write": 0}}})
    publish("session.status", **base, status={"type": "idle"})
class Handler(http.server.BaseHTTPRequestHandler):
    def log_message(self, *args): pass
    def authorized(self):
        expected = "Basic " + base64.b64encode(("opencode:" + password).encode()).decode()
        if password and self.headers.get("Authorization") != expected:
            self.send_response(401); self.end_headers(); return False
        return True
    def body(self):
        n = int(self.headers.get("Content-Length") or 0)
        return json.loads(self.rfile.read(n) or b"null") if n else None
    def send_json(self, obj, code=200):
        raw = json.dumps(obj).encode()
        self.send_response(code); self.send_header("Content-Type", "application/json"); self.send_header("Content-Length", str(len(raw))); self.end_headers(); self.wfile.write(raw)
    def do_GET(self):
        if not self.authorized(): return
        if self.path == "/event":
            q = queue.Queue()
            with lock: listeners.append(q)
            self.send_response(200); self.send_header("Content-Type", "text/event-stream"); self.end_headers()
            self.wfile.write(b'data: {"type":"server.connected","properties":{}}\n\n'); self.wfile.flush()
            try:
                while True:
                    self.wfile.write(("data: " + json.dumps(q.get()) + "\n\n").encode()); self.wfile.flush()
            except OSError:
                return
        if self.path.startswith("/session/"):
            if self.path.endswith("ses_gone"): return self.send_json({"error": "not found"}, 404)
            return self.send_json({"id": self.path.rsplit("/", 1)[1]})
        self.send_json({"healthy": True})
    def do_POST(self):
        if not self.authorized(): return
        body = self.body()
        if self.path == "/session":
            return self.send_json({"id": "ses_fake_live"})
        if self.path.endswith("/prompt_async"):
            session = self.path.split("/")[2]
            note(prompt=body)
            text = body["parts"][0]["text"]
            threading.Thread(target=scenario, args=(session, text), daemon=True).start()
            self.send_response(204); self.end_headers(); return
        if self.path.startswith("/permission/") or self.path.startswith("/question/"):
            request_id = self.path.split("/")[2]
            kind = self.path.rsplit("/", 1)[1]
            note(reply_to=request_id, action=kind, body=body)
            replies[request_id] = (body or {}).get("reply") if request_id.startswith("per_") else ((body or {}).get("answers") if kind == "reply" else "rejected")
            return self.send_json(True)
        self.send_json({})
class Server(http.server.ThreadingHTTPServer):
    def server_bind(self):
        # The stock version resolves the host name (`getfqdn`), which takes tens of seconds on macOS runners.
        import socketserver
        socketserver.TCPServer.server_bind(self)
        self.server_name, self.server_port = "127.0.0.1", self.socket.getsockname()[1]
server = Server(("127.0.0.1", 0), Handler)
print("opencode server listening on http://127.0.0.1:%d" % server.server_port, flush=True)
server.serve_forever()
'''


# A `claude -p --input-format stream-json` that asks the host before it acts, like the real CLI.
CLAUDE = r'''
import json, os, sys
sys.stdin.reconfigure(encoding="utf-8")  # the host writes UTF-8 whatever the Windows code page is
args = sys.argv[1:]
live = "--input-format" in args
log = os.environ.get("FAKE_CLI_LOG")
def note(**fields):
    if log:
        with open(log, "a", encoding="utf-8") as stream:
            stream.write(json.dumps({"tool": "claude", **fields}) + "\n")
def emit(record):
    print(json.dumps(record), flush=True)
first = sys.stdin.readline()
prompt = json.loads(first)["message"]["content"] if live else first + sys.stdin.read()
note(argv=args, prompt=prompt)
emit({"type": "system", "subtype": "init", "session_id": "claude-live-1"})
def ask(request_id, tool, data):
    emit({"type": "control_request", "request_id": request_id,
          "request": {"subtype": "can_use_tool", "tool_name": tool, "input": data, "tool_use_id": "tu-" + request_id}})
    line = sys.stdin.readline()
    if not line:
        sys.exit(3)
    response = json.loads(line)["response"]
    note(response=response["response"])
    return response["response"]
def say(text):
    emit({"type": "assistant", "message": {"content": [{"type": "text", "text": text}]}})
if "PERMISO" in prompt:
    decision = ask("r1", "Bash", {"command": "python3 -V", "description": "version"})
    say("hecho" if decision["behavior"] == "allow" else "denegado: " + decision.get("message", ""))
elif "PREGUNTA" in prompt:
    decision = ask("r2", "AskUserQuestion", {"questions": [{"question": "Color?", "header": "Color", "multiSelect": False,
                   "options": [{"label": "Rojo", "description": ""}, {"label": "Azul", "description": ""}]}]})
    say("respuesta: " + json.dumps(decision.get("updatedInput", {}).get("answers", decision), ensure_ascii=False))
else:
    say("eco: " + prompt.strip())
emit({"type": "result", "subtype": "success", "is_error": False, "usage": {"input_tokens": 10, "output_tokens": 2}, "permission_denials": []})
if live:
    sys.stdin.read()  # like the real CLI, wait for the next message until the host closes stdin
'''


# A `codex app-server` (JSON-RPC over stdio) that asks the host before it acts, like the real one.
CODEX = r'''
import json, os, sys
sys.stdin.reconfigure(encoding="utf-8")
args = sys.argv[1:]
log = os.environ.get("FAKE_CLI_LOG")
def note(**fields):
    if log:
        with open(log, "a", encoding="utf-8") as stream:
            stream.write(json.dumps({"tool": "codex", **fields}) + "\n")
def send(obj):
    print(json.dumps(obj), flush=True)
def read():
    line = sys.stdin.readline()
    return json.loads(line) if line else None
note(argv=args)
if args[:1] != ["app-server"]:
    sys.exit(2)
def item(kind, identifier, **fields):
    return {"type": kind, "id": identifier, **fields}
next_server_id = [0]
def ask(method, params):
    request_id = next_server_id[0]; next_server_id[0] += 1
    send({"id": request_id, "method": method, "params": params})
    while True:
        message = read()
        if message is None:
            sys.exit(3)
        if "id" in message and "method" not in message:
            note(response=message["result"])
            return message["result"]
thread = "thread-fake-1"
while True:
    message = read()
    if message is None:
        break
    method = message.get("method")
    if method == "initialize":
        note(initialize=message["params"])
        send({"id": message["id"], "result": {"userAgent": "fake"}})
    elif method in ("thread/start", "thread/resume"):
        note(thread=method, params=message["params"])
        send({"id": message["id"], "result": {"thread": {"id": message["params"].get("threadId", thread)}}})
    elif method == "turn/start":
        params = message["params"]
        note(turn=params)
        prompt = params["input"][0]["text"]
        send({"id": message["id"], "result": {"turn": {"id": "turn-1", "status": "inProgress"}}})
        send({"method": "turn/started", "params": {"threadId": thread, "turn": {"id": "turn-1"}}})
        text = "eco: " + prompt.strip().splitlines()[-1]
        base = {"threadId": thread, "turnId": "turn-1"}
        if "PERMISO" in prompt:
            result = ask("item/commandExecution/requestApproval", {**base, "itemId": "c1", "command": "/bin/bash -lc 'python3 -V'",
                         "reason": "version", "startedAtMs": 1})
            send({"method": "item/completed", "params": {**base, "item": item("commandExecution", "c1", command="/bin/bash -lc 'python3 -V'",
                  status="completed" if result["decision"] != "decline" else "declined", aggregatedOutput="Python 3.11\n", exitCode=0)}})
            text = "decision: " + json.dumps(result["decision"])
        elif "ARCHIVO" in prompt:
            result = ask("item/fileChange/requestApproval", {**base, "itemId": "f1", "reason": "fuera del proyecto", "startedAtMs": 1})
            text = "decision: " + json.dumps(result["decision"])
        elif "RED" in prompt:
            result = ask("item/permissions/requestApproval", {**base, "itemId": "p1", "cwd": "/x", "startedAtMs": 1,
                         "permissions": {"network": {"enabled": True}}})
            text = "permisos: " + json.dumps(result)
        elif "PREGUNTA" in prompt:
            result = ask("item/tool/requestUserInput", {**base, "itemId": "q1", "isBlocking": True, "questions": [
                {"id": "color", "header": "Color", "question": "Color?", "options": [{"label": "Rojo", "description": "r"}, {"label": "Azul", "description": "a"}]}]})
            text = "respuesta: " + json.dumps(result["answers"], ensure_ascii=False)
        send({"method": "item/completed", "params": {**base, "item": item("agentMessage", "m1", text=text, phase="final_answer")}})
        usage = {"inputTokens": 120, "outputTokens": 8, "cachedInputTokens": 0, "totalTokens": 128, "reasoningOutputTokens": 0, "cacheWriteInputTokens": 0}
        send({"method": "thread/tokenUsage/updated", "params": {**base, "tokenUsage": {"total": {**usage, "inputTokens": 1120}, "last": usage, "modelContextWindow": 200000}}})
        send({"method": "turn/completed", "params": {"threadId": thread, "turn": {"id": "turn-1", "status": "completed", "error": None}}})
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
