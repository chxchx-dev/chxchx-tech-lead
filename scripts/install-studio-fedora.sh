#!/usr/bin/env bash
set -Eeuo pipefail

OWNER="chxchx-dev"
REPOSITORY="chxchx-tech-lead"
BRANCH="dev"
ARTIFACT_NAME="chxchx-studio-fedora"
API="https://api.github.com/repos/${OWNER}/${REPOSITORY}"

fail() {
  printf 'Error: %s\n' "$*" >&2
  exit 1
}

if [[ "$(uname -s)" != "Linux" ]] || [[ ! -r /etc/fedora-release ]]; then
  fail "Este instalador requiere Fedora Linux."
fi
if [[ "$(uname -m)" != "x86_64" ]]; then
  fail "El artefacto de prueba actual se construye para x86_64; esta máquina es $(uname -m)."
fi
if [[ "${EUID}" -eq 0 ]]; then
  fail "Ejecuta el instalador como usuario normal; solicitará sudo para dependencias."
fi
for tool in curl python3 tar sha256sum; do
  command -v "$tool" >/dev/null 2>&1 || fail "Falta '$tool'. Instálalo y vuelve a ejecutar este instalador."
done

WORK_DIR=$(mktemp -d)
trap 'rm -rf "$WORK_DIR"' EXIT

printf 'Buscando el último push de %s/%s en %s…\n' "$OWNER" "$REPOSITORY" "$BRANCH"
curl --fail --silent --show-error --location \
  -H 'Accept: application/vnd.github+json' \
  -H 'X-GitHub-Api-Version: 2022-11-28' \
  "${API}/actions/workflows/ci.yml/runs?branch=${BRANCH}&per_page=50" \
  -o "$WORK_DIR/runs.json"

RUN_INFO=$(python3 - "$WORK_DIR/runs.json" "$OWNER/$REPOSITORY" "$BRANCH" <<'PY'
import json
import sys

with open(sys.argv[1], encoding="utf-8") as stream:
    runs = json.load(stream).get("workflow_runs", [])
for run in runs:
    repository = (run.get("head_repository") or {}).get("full_name", "").casefold()
    if (
        run.get("event") == "push"
        and run.get("head_branch") == sys.argv[3]
        and repository == sys.argv[2].casefold()
    ):
        print(run["id"], run["head_sha"])
        break
else:
    raise SystemExit("No hay un push propio a dev disponible todavía.")
PY
) || fail "No se encontró el último push de dev. Revisa la pestaña Actions de GitHub."
read -r RUN_ID COMMIT_SHA <<< "$RUN_INFO"

curl --fail --silent --show-error --location \
  -H 'Accept: application/vnd.github+json' \
  -H 'X-GitHub-Api-Version: 2022-11-28' \
  "${API}/actions/runs/${RUN_ID}/artifacts?per_page=100" \
  -o "$WORK_DIR/artifacts.json"

ARTIFACT_INFO=$(python3 - "$WORK_DIR/artifacts.json" "$ARTIFACT_NAME" <<'PY'
import json
import sys

with open(sys.argv[1], encoding="utf-8") as stream:
    artifacts = json.load(stream).get("artifacts", [])
for artifact in artifacts:
    if artifact.get("name") == sys.argv[2] and not artifact.get("expired"):
        print(artifact["id"], artifact.get("digest") or "-")
        break
else:
    raise SystemExit("El job Fedora todavía no publicó el paquete de este push.")
PY
) || fail "El artefacto Fedora de este push todavía no está disponible. Espera a que termine su job en Actions."
read -r ARTIFACT_ID ARTIFACT_DIGEST <<< "$ARTIFACT_INFO"

printf 'Descargando Studio desde el commit %s…\n' "${COMMIT_SHA:0:12}"
curl --fail --silent --show-error --location \
  -H 'Accept: application/vnd.github+json' \
  -H 'X-GitHub-Api-Version: 2022-11-28' \
  "${API}/actions/artifacts/${ARTIFACT_ID}/zip" \
  -o "$WORK_DIR/studio-artifact.zip"
if [[ "$ARTIFACT_DIGEST" == sha256:* ]]; then
  printf '%s  %s\n' "${ARTIFACT_DIGEST#sha256:}" "$WORK_DIR/studio-artifact.zip" \
    | sha256sum --check --status || fail "El checksum del artefacto no coincide."
fi

mkdir "$WORK_DIR/artifact"
python3 - "$WORK_DIR/studio-artifact.zip" "$WORK_DIR/artifact" <<'PY'
from pathlib import Path
import sys
from zipfile import ZipFile

destination = Path(sys.argv[2]).resolve()
with ZipFile(sys.argv[1]) as archive:
    for member in archive.infolist():
        target = (destination / member.filename).resolve()
        if not target.is_relative_to(destination):
            raise SystemExit(f"Ruta inválida en el artefacto: {member.filename}")
    archive.extractall(destination)
PY
PACKAGE=$(find "$WORK_DIR/artifact" -type f -name '*.tar.gz' -print -quit)
[[ -n "$PACKAGE" ]] || fail "El artefacto no contiene el paquete TGZ de Studio."

if ! command -v dnf >/dev/null 2>&1; then
  fail "No se encontró dnf para instalar las dependencias de Fedora."
fi
command -v sudo >/dev/null 2>&1 || fail "Instala sudo para obtener las dependencias de ejecución."
DNF=(sudo dnf)
"${DNF[@]}" install -y qt6-qtbase qt6-qtbase-gui qt6-qt5compat

APP_ROOT="$HOME/.local/opt/chxchx-studio/dev-${COMMIT_SHA:0:12}"
if [[ -e "$APP_ROOT" && ! -f "$APP_ROOT/.chxchx-managed" ]]; then
  fail "La ruta $APP_ROOT ya existe y no parece administrada por ChxChx. No la modifiqué."
fi
mkdir -p "$APP_ROOT"
touch "$APP_ROOT/.chxchx-managed"
tar -xzf "$PACKAGE" -C "$APP_ROOT" --strip-components=1
APP_BINARY=$(find "$APP_ROOT" -type f -name chxchx-studio -perm /111 -print -quit)
[[ -n "$APP_BINARY" ]] || fail "No encontré el ejecutable de Studio dentro del paquete."

if ! command -v uv >/dev/null 2>&1; then
  printf 'Instalando uv para el bridge CLI…\n'
  curl --fail --silent --show-error --location https://astral.sh/uv/install.sh | sh
fi
UV_BIN=$(command -v uv || true)
if [[ -z "$UV_BIN" && -x "$HOME/.local/bin/uv" ]]; then
  UV_BIN="$HOME/.local/bin/uv"
fi
[[ -x "$UV_BIN" ]] || fail "No encontré uv después de instalarlo."

curl --fail --silent --show-error --location \
  "https://api.github.com/repos/${OWNER}/${REPOSITORY}/tarball/${COMMIT_SHA}" \
  -o "$WORK_DIR/source.tar.gz"
mkdir "$WORK_DIR/source"
tar -xzf "$WORK_DIR/source.tar.gz" -C "$WORK_DIR/source" --strip-components=1
"$UV_BIN" venv "$APP_ROOT/cli-env"
"$UV_BIN" pip install --python "$APP_ROOT/cli-env/bin/python" "$WORK_DIR/source"

BIN_DIR="$HOME/.local/bin"
mkdir -p "$BIN_DIR"
LAUNCHER="$BIN_DIR/chxchx-studio-dev"
if [[ -e "$LAUNCHER" ]] && ! grep -q 'CHXCHX_STUDIO_DEV_LAUNCHER' "$LAUNCHER"; then
  fail "Ya existe $LAUNCHER y no parece administrado por ChxChx. No lo sobrescribí."
fi
cat > "$LAUNCHER" <<EOF
#!/usr/bin/env bash
# CHXCHX_STUDIO_DEV_LAUNCHER
export PATH="$BIN_DIR:\$PATH"
export CHXCHX_TECH_CLI="$APP_ROOT/cli-env/bin/chxchx-tech"
exec "$APP_BINARY" "\$@"
EOF
chmod 755 "$LAUNCHER"

printf '\nStudio quedó instalado desde dev (%s).\n' "${COMMIT_SHA:0:12}"
printf 'Para abrirlo con un proyecto: %s /ruta/al/proyecto\n' "$LAUNCHER"
