#!/bin/bash
# Test environment for the web client: isolated home, one trusted project, a fake Codex and a fake
# agent service, and the node on a fixed port (47831). The real node binary comes from CHXCHX_BIN
# (built with --features node and the client inside). Usage: env.sh start|stop|pair [role]|restart
set -u
HERE=$(cd "$(dirname "$0")" && pwd)
BIN=${CHXCHX_BIN:-$HERE/../../../target/debug/chxchx-tech}
W=$HERE/world
PORT=47831
export CHXCHX_TECH_HOME="$W/home"
export CHXCHX_AGENT_ADAPTER="$W/adapter"
export PATH="$W/bin:/usr/bin:/bin"

case "${1:-}" in
start)
  "$0" stop >/dev/null 2>&1
  rm -rf "$W"; mkdir -p "$W/bin" "$W/home" "$W/demo"
  printf '#!/bin/sh\necho codex 0.0.0\n' > "$W/bin/codex"; chmod +x "$W/bin/codex"
  cat > "$W/adapter" <<'PY'
#!/usr/bin/env python3
import json, select, sys, time
def emit(**event):
    print(json.dumps({"schema": "chxchx.agent-event", "schema_version": 1, **event}), flush=True)
if len(sys.argv) > 1 and sys.argv[1] == "contract":
    print(json.dumps({"schema": "chxchx.agent-adapter", "contract_version": 1, "version": "fake", "providers": ["codex", "opencode"]}))
    sys.exit(0)
request = json.loads(sys.stdin.readline())
# The node puts its own instructions before what the person wrote: the last line is theirs.
prompt = request.get("prompt", "").strip().splitlines()[-1].lower()
emit(type="session", session_id="sess-1")
emit(type="message", role="assistant", text="Voy a mirar el proyecto.")
emit(type="tool", id="t1", title="ls -la", status="running")
emit(type="tool", id="t1", status="completed", output="README.md\nsrc")
if "permiso" in prompt:
    emit(type="permission", id="p1", request_id="req-1", tool="Bash", summary="cargo test --all", detail="cargo test --all", certain=True,
         choices=[{"label": "Permitir esta vez", "scope": "once"}, {"label": "Permitir en esta sesión", "scope": "session"}])
    answer = json.loads(sys.stdin.readline())
    emit(type="message", role="assistant", text="Respuesta recibida: " + str(answer.get("behavior")))
elif "pregunta" in prompt:
    emit(type="question", id="qq", request_id="q-1", questions=[{"question": "¿Qué base de datos usar?", "header": "BD", "multiSelect": False,
         "options": [{"label": "PostgreSQL", "description": "Servidor"}, {"label": "SQLite", "description": "Archivo"}]}])
    answer = json.loads(sys.stdin.readline())
    emit(type="message", role="assistant", text="Elegiste: " + json.dumps(answer.get("answers"), ensure_ascii=False))
if "lento" in prompt:
    for step in range(120):
        ready, _, _ = select.select([sys.stdin], [], [], 1)
        if ready and sys.stdin.readline().strip() == "cancel":
            emit(type="adapter_done", exit_code=0, cancelled=True, session_id="sess-1")
            sys.exit(0)
        emit(type="message", role="assistant", text="paso %d" % step)
emit(type="message", role="assistant", text="Listo.")
emit(type="adapter_done", exit_code=0, cancelled=False, session_id="sess-1")
PY
  chmod +x "$W/adapter"
  ( cd "$W/demo" && "$BIN" init . --apply --minimal >/dev/null && "$BIN" workspace trust . --apply >/dev/null )
  ( cd "$W" && exec "$BIN" node start --port $PORT ) > "$W/node.log" 2>&1 < /dev/null &
  for _ in $(seq 50); do curl -s -m 1 "http://127.0.0.1:$PORT/api/v1/health" >/dev/null && break; sleep 0.2; done
  curl -s "http://127.0.0.1:$PORT/api/v1/health"; echo
  ;;
pair)
  "$BIN" node pair --role "${2:-operator}" | sed -n 's/^Código de vinculación: //p'
  ;;
cli)
  # Any command of the product, with this environment's home (e.g. `cli identity set --name ANA`).
  "$BIN" "${@:2}"
  ;;
stop)
  "$BIN" node stop 2>&1 | tail -1
  pkill -f "$W/adapter" 2>/dev/null
  true
  ;;
restart)
  ( cd "$W" && exec "$BIN" node start --port $PORT ) >> "$W/node.log" 2>&1 < /dev/null &
  for _ in $(seq 50); do curl -s -m 1 "http://127.0.0.1:$PORT/api/v1/health" >/dev/null && break; sleep 0.2; done
  curl -s "http://127.0.0.1:$PORT/api/v1/health"; echo
  ;;
esac
