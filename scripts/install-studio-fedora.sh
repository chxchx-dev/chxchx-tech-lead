#!/usr/bin/env bash
set -Eeuo pipefail

OWNER="chxchx-dev"
REPOSITORY="chxchx-tech-lead"
PREVIEW_TAG="studio-fedora-dev"
API="https://api.github.com/repos/${OWNER}/${REPOSITORY}"
DOWNLOAD="https://github.com/${OWNER}/${REPOSITORY}/releases/download/${PREVIEW_TAG}"

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
CACHE_BUSTER=$(date +%s)

printf 'Buscando el último paquete Fedora publicado desde dev…\n'
curl --fail --silent --show-error --location \
  -H 'Cache-Control: no-cache' \
  "${DOWNLOAD}/chxchx-studio-fedora.json?t=${CACHE_BUSTER}" \
  -o "$WORK_DIR/manifest.json"

read -r COMMIT_SHA PACKAGE_NAME EXPECTED_SHA256 < <(python3 - "$WORK_DIR/manifest.json" <<'PY'
import json
import re
import sys

with open(sys.argv[1], encoding="utf-8") as stream:
    manifest = json.load(stream)
commit = manifest.get("commit", "")
asset = manifest.get("asset", "")
digest = manifest.get("sha256", "")
if not re.fullmatch(r"[0-9a-f]{40}", commit):
    raise SystemExit("El manifiesto contiene un commit inválido.")
if asset != "chxchx-studio-fedora.tar.gz":
    raise SystemExit("El manifiesto apunta a un paquete inesperado.")
if not re.fullmatch(r"[0-9a-f]{64}", digest):
    raise SystemExit("El manifiesto contiene un SHA-256 inválido.")
print(commit, asset, digest)
PY
) || fail "No se pudo leer el manifiesto del preview Fedora. Espera a que CI publique el release."

printf 'Descargando Studio desde el commit %s…\n' "${COMMIT_SHA:0:12}"
curl --fail --silent --show-error --location \
  -H 'Cache-Control: no-cache' \
  "${DOWNLOAD}/${PACKAGE_NAME}?t=${CACHE_BUSTER}" \
  -o "$WORK_DIR/$PACKAGE_NAME"
printf '%s  %s\n' "$EXPECTED_SHA256" "$WORK_DIR/$PACKAGE_NAME" \
  | sha256sum --check --status || fail "El SHA-256 del paquete no coincide."

if ! command -v dnf >/dev/null 2>&1; then
  fail "No se encontró dnf para instalar las dependencias de Fedora."
fi
command -v sudo >/dev/null 2>&1 || fail "Instala sudo para obtener las dependencias de ejecución."
sudo dnf install -y qt6-qtbase qt6-qtbase-gui qt6-qt5compat

APP_ROOT="$HOME/.local/opt/chxchx-studio/dev-${COMMIT_SHA:0:12}"
if [[ -e "$APP_ROOT" && ! -f "$APP_ROOT/.chxchx-managed" ]]; then
  fail "La ruta $APP_ROOT ya existe y no parece administrada por ChxChx. No la modifiqué."
fi
mkdir -p "$APP_ROOT"
touch "$APP_ROOT/.chxchx-managed"
tar -xzf "$WORK_DIR/$PACKAGE_NAME" -C "$APP_ROOT" --strip-components=1
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
  "${API}/tarball/${COMMIT_SHA}" \
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
