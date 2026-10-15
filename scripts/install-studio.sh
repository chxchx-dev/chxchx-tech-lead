#!/usr/bin/env bash
set -Eeuo pipefail

OWNER="chxchx-dev"
REPOSITORY="chxchx-tech-lead"
PREVIEW_TAG="studio-dev-preview"
BASE="https://github.com/${OWNER}/${REPOSITORY}/releases/download/${PREVIEW_TAG}"

fail() {
  printf 'Error: %s\n' "$*" >&2
  exit 1
}

DRY_RUN=false
for argument in "$@"; do
  case "$argument" in
    --dry-run) DRY_RUN=true ;;
    -h|--help)
      printf 'Uso: install-studio.sh [--dry-run]\nInstala el preview más reciente de ChxChx Studio.\n'
      exit 0
      ;;
    *) fail "Opción no reconocida: $argument" ;;
  esac
done

[[ "$(id -u)" -ne 0 ]] || fail "Ejecuta el instalador como usuario normal; puede pedir sudo para librerías del sistema."
OS="$(uname -s)"
ARCH="$(uname -m)"
case "$OS/$ARCH" in
  Darwin/arm64) PLATFORM="macos-arm64"; PACKAGE="chxchx-studio-macos-arm64.dmg" ;;
  Darwin/x86_64) PLATFORM="macos-x86_64"; PACKAGE="chxchx-studio-macos-x86_64.dmg" ;;
  Linux/x86_64)
    if [[ -r /etc/os-release ]]; then
      . /etc/os-release
    fi
    case "${ID:-}" in
      ubuntu) PLATFORM="linux-ubuntu-x86_64"; PACKAGE="chxchx-studio-ubuntu-x86_64.tar.gz" ;;
      fedora) PLATFORM="linux-fedora-x86_64"; PACKAGE="chxchx-studio-fedora-x86_64.tar.gz" ;;
      *) fail "El preview soporta Ubuntu y Fedora x86_64; sistema detectado: ${ID:-desconocido}." ;;
    esac
    ;;
  *) fail "Plataforma sin paquete publicado: ${OS}/${ARCH}." ;;
esac

for tool in curl awk; do
  command -v "$tool" >/dev/null 2>&1 || fail "Falta '$tool'. Instálalo y vuelve a ejecutar el instalador."
done
if command -v sha256sum >/dev/null 2>&1; then
  sha_file() { sha256sum "$1" | awk '{print $1}'; }
elif command -v shasum >/dev/null 2>&1; then
  sha_file() { shasum -a 256 "$1" | awk '{print $1}'; }
else
  fail "Se requiere sha256sum o shasum para comprobar las descargas."
fi

WORK_DIR="$(mktemp -d)"
MOUNT_DIR=""
STAGE=""
cleanup() {
  if [[ -n "$MOUNT_DIR" ]]; then
    hdiutil detach "$MOUNT_DIR" >/dev/null 2>&1 || true
  fi
  if [[ -n "$STAGE" && -d "$STAGE" ]]; then
    rm -rf "$STAGE"
  fi
  rm -rf "$WORK_DIR"
}
trap cleanup EXIT
CACHE_BUSTER="$(date +%s)"
MANIFEST="$WORK_DIR/studio-preview.txt"
curl --fail --silent --show-error --location \
  -H 'Cache-Control: no-cache' "${BASE}/studio-preview.txt?t=${CACHE_BUSTER}" -o "$MANIFEST"

manifest_value() {
  awk -F= -v key="$1" '$1 == key { sub(/^[^=]*=/, ""); print; count++ } END { if (count != 1) exit 1 }' "$MANIFEST"
}
SCHEMA="$(manifest_value schema_version)" || fail "El manifiesto no contiene schema_version válido."
COMMIT_SHA="$(manifest_value commit)" || fail "El manifiesto no contiene el commit."
VERSION="$(manifest_value version)" || fail "El manifiesto no contiene la versión."
PACKAGE_SHA="$(manifest_value "sha256.${PLATFORM}")" || fail "No hay checksum para ${PLATFORM}."
WHEEL_NAME="$(manifest_value asset.cli-wheel)" || fail "No hay wheel del bridge CLI."
WHEEL_SHA="$(manifest_value sha256.cli-wheel)" || fail "No hay checksum del bridge CLI."
[[ "$SCHEMA" == "1" ]] || fail "Versión de manifiesto no compatible: ${SCHEMA}."
[[ "$COMMIT_SHA" =~ ^[0-9a-f]{40}$ ]] || fail "El manifiesto contiene un commit inválido."
[[ "$VERSION" =~ ^[0-9A-Za-z.+-]+$ ]] || fail "El manifiesto contiene una versión inválida."
[[ "$PACKAGE_SHA" =~ ^[0-9a-f]{64}$ && "$WHEEL_SHA" =~ ^[0-9a-f]{64}$ ]] || fail "El manifiesto contiene un SHA-256 inválido."
EXPECTED_PACKAGE="$(manifest_value "asset.${PLATFORM}")" || fail "No hay paquete para ${PLATFORM}."
[[ "$EXPECTED_PACKAGE" == "$PACKAGE" ]] || fail "El manifiesto apunta a un paquete inesperado."
[[ "$WHEEL_NAME" =~ ^chxchx_tech_lead-[A-Za-z0-9.+!_-]+-py3-none-any\.whl$ ]] || fail "Nombre inesperado para el wheel del CLI."

SHORT_COMMIT="${COMMIT_SHA:0:12}"
curl --fail --silent --show-error --location "${BASE}/${PACKAGE}?t=${CACHE_BUSTER}" -o "$WORK_DIR/$PACKAGE"
curl --fail --silent --show-error --location "${BASE}/${WHEEL_NAME}?t=${CACHE_BUSTER}" -o "$WORK_DIR/$WHEEL_NAME"
[[ "$(sha_file "$WORK_DIR/$PACKAGE")" == "$PACKAGE_SHA" ]] || fail "El SHA-256 de Studio no coincide."
[[ "$(sha_file "$WORK_DIR/$WHEEL_NAME")" == "$WHEEL_SHA" ]] || fail "El SHA-256 del CLI no coincide."

if [[ "$OS" == "Darwin" ]]; then
  APP_PARENT="$HOME/Applications"
  APP_ROOT="$APP_PARENT/ChxChx Studio Dev-${SHORT_COMMIT}.app"
  APP_LINK="$APP_PARENT/ChxChx Studio Dev.app"
  CLI_ROOT="$APP_ROOT/Contents"
  APP_BINARY="$APP_ROOT/Contents/MacOS/chxchx-studio"
  CLI_BINARY="$CLI_ROOT/.venv/bin/chxchx-tech"
  LAUNCHER="$HOME/.local/bin/chxchx-studio-dev"
else
  APP_PARENT="$HOME/.local/opt/chxchx-studio"
  APP_ROOT="$APP_PARENT/dev-${SHORT_COMMIT}"
  CLI_ROOT="$APP_ROOT"
  APP_BINARY="$APP_ROOT/bin/chxchx-studio"
  CLI_BINARY="$CLI_ROOT/.venv/bin/chxchx-tech"
  LAUNCHER="$HOME/.local/bin/chxchx-studio-dev"
fi

if [[ -e "$LAUNCHER" ]] && ! grep -q 'CHXCHX_STUDIO_DEV_LAUNCHER' "$LAUNCHER"; then
  fail "Ya existe $LAUNCHER y no está administrado por ChxChx; no lo sobrescribí."
fi
if [[ "$OS" == "Darwin" && ( -e "$APP_LINK" || -L "$APP_LINK" ) ]]; then
  [[ -L "$APP_LINK" ]] || fail "Ya existe $APP_LINK y no es el enlace administrado por ChxChx."
  LINK_TARGET="$(readlink "$APP_LINK")"
  case "$LINK_TARGET" in
    "$HOME/Applications/ChxChx Studio Dev-"*.app) ;;
    *) fail "El enlace $APP_LINK apunta a una ruta ajena; no lo cambié." ;;
  esac
fi

printf 'Preview %s (%s) · %s\n' "$VERSION" "$SHORT_COMMIT" "$PLATFORM"
printf 'Instalación de Studio: %s\nCLI bridge aislado: %s/.venv\n' "$APP_ROOT" "$CLI_ROOT"
if [[ "$DRY_RUN" == true ]]; then
  printf 'DRY RUN: descargas y checksums válidos; no se instalaron paquetes ni se escribieron archivos fuera de /tmp.\n'
  exit 0
fi

if [[ "$OS" == "Linux" ]]; then
  command -v sudo >/dev/null 2>&1 || fail "Instala sudo para instalar las bibliotecas de ejecución."
  if [[ "$PLATFORM" == "linux-fedora-x86_64" ]]; then
    command -v dnf >/dev/null 2>&1 || fail "No se encontró dnf."
    sudo dnf install -y qt6-qtbase qt6-qtbase-gui qt6-qt5compat
  else
    command -v apt-get >/dev/null 2>&1 || fail "No se encontró apt-get."
    sudo apt-get update
    sudo apt-get install -y libegl1 libgl1 libxkbcommon-x11-0 libxcb-cursor0 \
      libxcb-icccm4 libxcb-image0 libxcb-keysyms1 libxcb-randr0 libxcb-render-util0 \
      libxcb-shape0 libxcb-xinerama0 libxcb-xkb1
  fi
fi

mkdir -p "$APP_PARENT"
if [[ -e "$APP_ROOT" ]]; then
  [[ -f "$APP_ROOT/.chxchx-managed" ]] || fail "La ruta $APP_ROOT ya existe y no está administrada por ChxChx."
  [[ -x "$APP_BINARY" ]] || fail "La instalación existente está incompleta: falta $APP_BINARY. Conservé los archivos para revisión."
else
  STAGE="$(mktemp -d "$APP_PARENT/.chxchx-stage.XXXXXX")"
  if [[ "$OS" == "Darwin" ]]; then
    MOUNT_DIR="$WORK_DIR/mount"
    mkdir -p "$MOUNT_DIR"
    hdiutil attach "$WORK_DIR/$PACKAGE" -readonly -nobrowse -mountpoint "$MOUNT_DIR" >/dev/null
    APP_BUNDLE="$(find "$MOUNT_DIR" -maxdepth 2 -type d -name '*.app' -print -quit)"
    [[ -n "$APP_BUNDLE" ]] || fail "No se encontró la app dentro del DMG."
    ditto "$APP_BUNDLE" "$STAGE/ChxChx Studio Dev-${SHORT_COMMIT}.app"
    hdiutil detach "$MOUNT_DIR" >/dev/null
    MOUNT_DIR=""
    STAGED_APP="$STAGE/ChxChx Studio Dev-${SHORT_COMMIT}.app"
    [[ -x "$STAGED_APP/Contents/MacOS/chxchx-studio" ]] || fail "El paquete macOS no contiene el ejecutable esperado."
    touch "$STAGED_APP/.chxchx-managed"
    mv "$STAGED_APP" "$APP_ROOT"
  else
    tar -xzf "$WORK_DIR/$PACKAGE" -C "$STAGE" --strip-components=1
    [[ -x "$STAGE/bin/chxchx-studio" ]] || fail "El paquete Linux no contiene bin/chxchx-studio."
    touch "$STAGE/.chxchx-managed"
    mv "$STAGE" "$APP_ROOT"
  fi
  rmdir "$STAGE" 2>/dev/null || true
  STAGE=""
fi
if [[ "$OS" == "Darwin" ]]; then
  ln -sfn "$APP_ROOT" "$APP_LINK"
fi

if ! command -v uv >/dev/null 2>&1 && [[ ! -x "$HOME/.local/bin/uv" ]]; then
  curl --fail --silent --show-error --location https://astral.sh/uv/install.sh | sh
fi
UV_BIN="$(command -v uv || true)"
[[ -n "$UV_BIN" ]] || UV_BIN="$HOME/.local/bin/uv"
[[ -x "$UV_BIN" ]] || fail "No se encontró uv después de instalarlo."
"$UV_BIN" venv --python 3.11 "$CLI_ROOT/.venv"
"$UV_BIN" pip install --python "$CLI_ROOT/.venv/bin/python" "$WORK_DIR/$WHEEL_NAME"
[[ -x "$CLI_BINARY" ]] || fail "El wheel no dejó el comando chxchx-tech esperado."

mkdir -p "$(dirname "$LAUNCHER")"
LAUNCHER_TMP="$LAUNCHER.tmp.$$"
if [[ "$OS" == "Darwin" ]]; then
  cat > "$LAUNCHER_TMP" <<EOF
#!/usr/bin/env bash
# CHXCHX_STUDIO_DEV_LAUNCHER
APP_ROOT="\$HOME/Applications/ChxChx Studio Dev.app"
export PATH="\$HOME/.local/bin:\$PATH"
export CHXCHX_TECH_CLI="\$APP_ROOT/Contents/.venv/bin/chxchx-tech"
exec "\$APP_ROOT/Contents/MacOS/chxchx-studio" "\$@"
EOF
else
  cat > "$LAUNCHER_TMP" <<EOF
#!/usr/bin/env bash
# CHXCHX_STUDIO_DEV_LAUNCHER
APP_ROOT="\$HOME/.local/opt/chxchx-studio/dev-${SHORT_COMMIT}"
export PATH="\$HOME/.local/bin:\$PATH"
export CHXCHX_TECH_CLI="\$APP_ROOT/.venv/bin/chxchx-tech"
exec "\$APP_ROOT/bin/chxchx-studio" "\$@"
EOF
fi
chmod 755 "$LAUNCHER_TMP"
mv "$LAUNCHER_TMP" "$LAUNCHER"

printf '\nStudio quedó instalado desde el preview %s.\n' "$SHORT_COMMIT"
printf 'Abre con: %s /ruta/al/proyecto\n' "$LAUNCHER"
if [[ "$OS" == "Darwin" ]]; then
  printf 'El preview no está firmado; macOS puede pedir confirmar Abrir una vez desde Finder.\n'
else
  printf 'Si el comando no aparece, agrega ~/.local/bin a PATH o abre una terminal nueva.\n'
fi
