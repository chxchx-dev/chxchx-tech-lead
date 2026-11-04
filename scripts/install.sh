#!/usr/bin/env bash
# Instala o actualiza ChxChx Studio y el comando `chxchx-tech` en Linux y macOS.
#
#   curl -fsSL https://github.com/chxchx-dev/chxchx-tech-lead/releases/latest/download/install.sh | bash
#   curl -fsSL .../install.sh | bash -s -- --dry-run
#
# No necesita clonar el repositorio. Descarga solo los paquetes de tu sistema,
# comprueba su SHA-256 contra el manifiesto del release e instala en tu usuario.
set -Eeuo pipefail

OWNER="chxchx-dev"
REPOSITORY="chxchx-tech-lead"
DEV_TAG="studio-dev-preview"
RELEASES="https://github.com/${OWNER}/${REPOSITORY}/releases"
MIN_GLIBC="2.35"
KEEP_VERSIONS=2

fail() { printf 'Error: %s\n' "$*" >&2; exit 1; }
note() { printf '%s\n' "$*"; }
warn() { printf 'Aviso: %s\n' "$*" >&2; }

usage() {
  cat <<'USAGE'
Uso: install.sh [opciones]

  --channel stable|dev   stable (por defecto): la última versión estable. dev: el preview de la rama dev.
  --version X.Y.Z        Instala esa versión estable (no combina con --channel dev).
  --update               Exige una instalación previa y la actualiza.
  --cli-only             Instala solo el comando chxchx-tech (sin la aplicación de escritorio).
  --uninstall            Quita lo que instaló ChxChx (no toca tus datos ni tus proyectos).
  --dry-run              Descarga y verifica, o lista lo que quitaría, sin cambiar nada.
  --no-deps              No instala bibliotecas del sistema con sudo (Linux).
  -h, --help             Muestra esta ayuda.
USAGE
}

CHANNEL="stable"
VERSION=""
DRY_RUN=false
UPDATE_ONLY=false
UNINSTALL=false
CLI_ONLY=false
INSTALL_DEPS=true
PRINT_URLS=false
while [[ $# -gt 0 ]]; do
  case "$1" in
    --dry-run) DRY_RUN=true ;;
    --update) UPDATE_ONLY=true ;;
    --uninstall) UNINSTALL=true ;;
    --cli-only) CLI_ONLY=true ;;
    --no-deps) INSTALL_DEPS=false ;;
    --print-urls) PRINT_URLS=true ;;
    --channel=*) CHANNEL="${1#*=}" ;;
    --channel) [[ $# -ge 2 ]] || fail "--channel necesita un valor (stable o dev)."; CHANNEL="$2"; shift ;;
    --version=*) VERSION="${1#*=}" ;;
    --version) [[ $# -ge 2 ]] || fail "--version necesita un valor (X.Y.Z)."; VERSION="$2"; shift ;;
    -h|--help) usage; exit 0 ;;
    *) fail "Opción no reconocida: $1. Usa --help." ;;
  esac
  shift
done
[[ "$CHANNEL" == "stable" || "$CHANNEL" == "dev" ]] || fail "Canal no válido: ${CHANNEL}. Usa stable o dev."
if [[ -n "$VERSION" ]]; then
  [[ "$CHANNEL" == "stable" ]] || fail "--version solo existe en el canal stable."
  [[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || fail "La versión debe tener la forma X.Y.Z (por ejemplo 0.5.0)."
fi
[[ "$UNINSTALL" == false || ( "$UPDATE_ONLY" == false && "$CLI_ONLY" == false && -z "$VERSION" ) ]] \
  || fail "--uninstall no se combina con --update, --cli-only ni --version."

[[ "$(id -u)" -ne 0 ]] || fail "Ejecútalo como usuario normal; pedirá sudo solo para bibliotecas del sistema."
[[ -n "${HOME:-}" && -d "$HOME" ]] || fail "No se pudo determinar tu carpeta personal (HOME)."

OS="$(uname -s)"
ARCH="$(uname -m)"
LOCAL_BIN="$HOME/.local/bin"
DATA_HOME="${XDG_DATA_HOME:-$HOME/.local/share}"
OPT_STUDIO="$HOME/.local/opt/chxchx-studio"
OPT_CLI="$HOME/.local/opt/chxchx-tech"
MAC_APPS="$HOME/Applications"

# ---------- Desinstalación -----------------------------------------------------------------
# is_managed_file ARCHIVO MARCA...: un archivo normal (no un enlace) que contiene alguna marca de ChxChx.
is_managed_file() {
  local file="$1" pattern
  local args=()
  shift
  for pattern in "$@"; do args+=(-e "$pattern"); done
  [[ -f "$file" && ! -L "$file" ]] && grep -q "${args[@]}" "$file" 2>/dev/null
}

REMOVED=0
remove_path() {
  if [[ "$DRY_RUN" == true ]]; then note "Se quitaría: $1"; else rm -rf -- "$1"; note "Quitado: $1"; fi
  REMOVED=$((REMOVED + 1))
}

do_uninstall() {
  local parent dir link
  # Solo carpetas con el marcador de ChxChx, nunca enlaces ni rutas ajenas.
  for parent in "$OPT_STUDIO" "$OPT_CLI"; do
    [[ -d "$parent" && ! -L "$parent" ]] || continue
    for dir in "$parent"/*; do
      [[ -d "$dir" && ! -L "$dir" && -f "$dir/.chxchx-managed" ]] && remove_path "$dir"
    done
  done
  if [[ -d "$MAC_APPS" ]]; then
    for dir in "$MAC_APPS"/ChxChx\ Studio*-*.app; do
      [[ -d "$dir" && ! -L "$dir" && -f "$dir/.chxchx-managed" ]] && remove_path "$dir"
    done
    for link in "$MAC_APPS/ChxChx Studio.app" "$MAC_APPS/ChxChx Studio Dev.app"; do
      [[ -L "$link" ]] || continue
      case "$(readlink "$link")" in "$MAC_APPS"/ChxChx\ Studio*-*.app) remove_path "$link" ;; esac
    done
  fi
  for link in "$LOCAL_BIN/chxchx-studio" "$LOCAL_BIN/chxchx-studio-dev"; do
    if is_managed_file "$link" CHXCHX_STUDIO_LAUNCHER CHXCHX_STUDIO_DEV_LAUNCHER; then remove_path "$link"; fi
  done
  if is_managed_file "$LOCAL_BIN/chxchx-tech" CHXCHX_CLI_SHIM; then remove_path "$LOCAL_BIN/chxchx-tech"; fi
  for link in "$DATA_HOME/applications/chxchx-studio.desktop" "$DATA_HOME/applications/chxchx-studio-dev.desktop"; do
    if is_managed_file "$link" CHXCHX_STUDIO_DESKTOP; then remove_path "$link"; fi
  done
  local icon="$DATA_HOME/icons/hicolor/256x256/apps/chxchx-studio.png"
  if [[ -f "$icon" && ! -L "$icon" ]] && ! ls "$DATA_HOME"/applications/chxchx-studio*.desktop >/dev/null 2>&1; then
    remove_path "$icon"
  fi
  if [[ "$DRY_RUN" == false ]]; then
    rmdir "$OPT_STUDIO" "$OPT_CLI" "$HOME/.local/opt" 2>/dev/null || true
    command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database "$DATA_HOME/applications" >/dev/null 2>&1 || true
  fi
  if [[ "$REMOVED" -eq 0 ]]; then
    note "No había nada instalado por ChxChx."
  elif [[ "$DRY_RUN" == true ]]; then
    note "DRY RUN: no se quitó nada."
  fi
  note "No se tocaron tus datos: ~/.chxchx-tech-lead (confianza, copias, registros) ni la carpeta .ai de tus proyectos."
  note "Las bibliotecas del sistema y uv se administran aparte. Para borrar tus datos de ChxChx: rm -rf ~/.chxchx-tech-lead"
}

if [[ "$UNINSTALL" == true ]]; then
  do_uninstall
  exit 0
fi

# ---------- Origen de las descargas -------------------------------------------------------------
TEST_BASE="${CHXCHX_TEST_BASE:-}"
PROTO="https"
if [[ -n "$TEST_BASE" ]]; then
  # Gancho para pruebas: solo un servidor local, y nunca para usuarios normales.
  [[ "$TEST_BASE" =~ ^http://(127\.0\.0\.1|localhost):[0-9]+(/[A-Za-z0-9._-]*)*$ ]] \
    || fail "CHXCHX_TEST_BASE solo admite http://127.0.0.1:PUERTO o http://localhost:PUERTO."
  PROTO="http"
  BASE="${TEST_BASE%/}"
  warn "Usando una base de pruebas (${BASE}); no es el origen oficial."
elif [[ "$CHANNEL" == "dev" ]]; then
  BASE="${RELEASES}/download/${DEV_TAG}"
elif [[ -n "$VERSION" ]]; then
  BASE="${RELEASES}/download/v${VERSION}"
else
  BASE="${RELEASES}/latest/download"
fi
if [[ "$PRINT_URLS" == true ]]; then
  printf '%s/studio-manifest.txt\n' "$BASE"
  exit 0
fi

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

# ---------- Plataforma ----------------------------------------------------------------------------
PLATFORM=""
PACKAGE=""
if [[ "$CLI_ONLY" == false ]]; then
  case "$OS/$ARCH" in
    Darwin/arm64) PLATFORM="macos-arm64"; PACKAGE="chxchx-studio-macos-arm64.dmg" ;;
    Darwin/x86_64) PLATFORM="macos-x86_64"; PACKAGE="chxchx-studio-macos-x86_64.dmg" ;;
    Linux/x86_64)
      # os-release define variables propias (VERSION, NAME…): se lee en un subshell para no pisar las del instalador.
      OS_RELEASE="${CHXCHX_OS_RELEASE:-/etc/os-release}"  # gancho para pruebas
      ID=""; ID_LIKE=""
      if [[ -r "$OS_RELEASE" ]]; then
        ID="$(. "$OS_RELEASE" >/dev/null 2>&1; printf '%s' "${ID:-}")"
        ID_LIKE="$(. "$OS_RELEASE" >/dev/null 2>&1; printf '%s' "${ID_LIKE:-}")"
      fi
      FLAVOR="${CHXCHX_LINUX_FLAVOR:-}"
      if [[ -z "$FLAVOR" ]]; then
        # El paquete incluye Qt; solo cambia el gestor con el que se instalan las bibliotecas gráficas.
        case " ${ID:-} ${ID_LIKE:-} " in
          *" fedora "*|*" rhel "*|*" centos "*|*" rocky "*|*" almalinux "*|*" nobara "*) FLAVOR="fedora" ;;
          *" ubuntu "*|*" debian "*|*" linuxmint "*|*" pop "*|*" elementary "*|*" zorin "*) FLAVOR="ubuntu" ;;
        esac
      fi
      case "$FLAVOR" in
        ubuntu) PLATFORM="linux-ubuntu-x86_64"; PACKAGE="chxchx-studio-ubuntu-x86_64.tar.gz" ;;
        fedora) PLATFORM="linux-fedora-x86_64"; PACKAGE="chxchx-studio-fedora-x86_64.tar.gz" ;;
        *) fail "No reconozco tu distribución (${ID:-desconocida}). Hay paquetes para la familia Debian/Ubuntu y Fedora/RHEL en x86_64. Prueba CHXCHX_LINUX_FLAVOR=ubuntu o fedora con --no-deps, o instala solo el comando con --cli-only." ;;
      esac
      GLIBC="$(getconf GNU_LIBC_VERSION 2>/dev/null | awk '{print $2}')"
      if [[ -n "$GLIBC" ]]; then
        LOWEST="$(printf '%s\n%s\n' "$MIN_GLIBC" "$GLIBC" | sort -V | head -n 1)"
        [[ "$LOWEST" == "$MIN_GLIBC" ]] || fail "Tu glibc es ${GLIBC} y el paquete necesita ${MIN_GLIBC} o posterior (Ubuntu 22.04, Debian 12, Fedora 36 o más nuevos). Usa --cli-only para instalar solo el comando."
      fi
      ;;
    *) fail "No hay paquete de escritorio para ${OS}/${ARCH}. Usa --cli-only para instalar solo el comando chxchx-tech." ;;
  esac
  [[ "$PACKAGE" != *.tar.gz ]] || command -v tar >/dev/null 2>&1 || fail "Falta 'tar'."
fi

WORK_DIR="$(mktemp -d)"
MOUNT_DIR=""
STAGE=""
cleanup() {
  if [[ -n "$MOUNT_DIR" ]]; then hdiutil detach "$MOUNT_DIR" >/dev/null 2>&1 || true; fi
  if [[ -n "$STAGE" && -d "$STAGE" ]]; then rm -rf "$STAGE"; fi
  rm -rf "$WORK_DIR"
}
trap cleanup EXIT

curl_get() { curl --fail --silent --show-error --location --retry 3 --retry-delay 2 --connect-timeout 20 --proto "=${PROTO}" "$@"; }

# ---------- Manifiesto ----------------------------------------------------------------------------
BUSTER="$(date +%s)"
MANIFEST="$WORK_DIR/studio-manifest.txt"
if ! curl_get -H 'Cache-Control: no-cache' "${BASE}/studio-manifest.txt?t=${BUSTER}" -o "$MANIFEST" 2>/dev/null; then
  # El preview anterior solo publicaba studio-preview.txt.
  [[ "$CHANNEL" == "dev" ]] \
    && curl_get -H 'Cache-Control: no-cache' "${BASE}/studio-preview.txt?t=${BUSTER}" -o "$MANIFEST" \
    || fail "No pude descargar el manifiesto de ${BASE}. ¿Hay conexión, y existe esa versión?"
fi
manifest_value() {
  awk -F= -v key="$1" '$1 == key { sub(/^[^=]*=/, ""); print; count++ } END { if (count != 1) exit 1 }' "$MANIFEST"
}
optional_value() { manifest_value "$1" 2>/dev/null || true; }

SCHEMA="$(manifest_value schema_version)" || fail "El manifiesto no contiene schema_version válido."
COMMIT_SHA="$(manifest_value commit)" || fail "El manifiesto no contiene el commit."
MANIFEST_VERSION="$(manifest_value version)" || fail "El manifiesto no contiene la versión."
WHEEL_NAME="$(manifest_value asset.cli-wheel)" || fail "No hay wheel del bridge CLI."
WHEEL_SHA="$(manifest_value sha256.cli-wheel)" || fail "No hay checksum del bridge CLI."
MANIFEST_CHANNEL="$(optional_value channel)"
[[ "$SCHEMA" == "1" ]] || fail "Versión de manifiesto no compatible: ${SCHEMA}."
[[ "$COMMIT_SHA" =~ ^[0-9a-f]{40}$ ]] || fail "El manifiesto contiene un commit inválido."
[[ "$MANIFEST_VERSION" =~ ^[0-9A-Za-z.+-]+$ ]] || fail "El manifiesto contiene una versión inválida."
[[ "$WHEEL_SHA" =~ ^[0-9a-f]{64}$ ]] || fail "El manifiesto contiene un SHA-256 inválido."
[[ "$WHEEL_NAME" =~ ^chxchx_tech_lead-[A-Za-z0-9.+!_-]+-py3-none-any\.whl$ ]] || fail "Nombre inesperado para el wheel del CLI."
if [[ -z "$TEST_BASE" && -n "$MANIFEST_CHANNEL" && "$MANIFEST_CHANNEL" != "$CHANNEL" ]]; then
  fail "El manifiesto es del canal ${MANIFEST_CHANNEL}, no de ${CHANNEL}."
fi
if [[ "$CHANNEL" == "stable" ]]; then
  [[ "$MANIFEST_VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || fail "La versión estable ${MANIFEST_VERSION} no tiene la forma X.Y.Z."
  [[ -z "$VERSION" || "$VERSION" == "$MANIFEST_VERSION" ]] || fail "Pediste ${VERSION} pero el release es ${MANIFEST_VERSION}."
fi
if [[ "$CLI_ONLY" == false ]]; then
  PACKAGE_SHA="$(manifest_value "sha256.${PLATFORM}")" || fail "No hay checksum para ${PLATFORM}."
  EXPECTED_PACKAGE="$(manifest_value "asset.${PLATFORM}")" || fail "No hay paquete para ${PLATFORM}."
  [[ "$PACKAGE_SHA" =~ ^[0-9a-f]{64}$ ]] || fail "El manifiesto contiene un SHA-256 inválido."
  [[ "$EXPECTED_PACKAGE" == "$PACKAGE" ]] || fail "El manifiesto apunta a un paquete inesperado."
fi

SHORT_COMMIT="${COMMIT_SHA:0:12}"
if [[ "$CHANNEL" == "dev" ]]; then
  ID="dev-${SHORT_COMMIT}"; SUFFIX="$SHORT_COMMIT"; APP_LABEL="ChxChx Studio Dev"; LAUNCHER_NAME="chxchx-studio-dev"
else
  ID="stable-${MANIFEST_VERSION}"; SUFFIX="$MANIFEST_VERSION"; APP_LABEL="ChxChx Studio"; LAUNCHER_NAME="chxchx-studio"
fi
LAUNCHER="$LOCAL_BIN/$LAUNCHER_NAME"
SHIM="$LOCAL_BIN/chxchx-tech"

# ---------- Rutas de la instalación --------------------------------------------------------------
APP_LINK=""
if [[ "$CLI_ONLY" == true ]]; then
  APP_PARENT="$OPT_CLI"; APP_ROOT="$APP_PARENT/$ID"; CLI_ROOT="$APP_ROOT"; APP_BINARY=""
elif [[ "$OS" == "Darwin" ]]; then
  APP_PARENT="$MAC_APPS"; APP_ROOT="$APP_PARENT/${APP_LABEL}-${SUFFIX}.app"; APP_LINK="$APP_PARENT/${APP_LABEL}.app"
  CLI_ROOT="$APP_ROOT/Contents"; APP_BINARY="$APP_ROOT/Contents/MacOS/chxchx-studio"
else
  APP_PARENT="$OPT_STUDIO"; APP_ROOT="$APP_PARENT/$ID"; CLI_ROOT="$APP_ROOT"; APP_BINARY="$APP_ROOT/bin/chxchx-studio"
fi
CLI_BINARY="$CLI_ROOT/.venv/bin/chxchx-tech"

# ---------- Comprobaciones previas (no se pisa nada ajeno) ----------------------------------------
if [[ "$CLI_ONLY" == false ]]; then
  if [[ -L "$LAUNCHER" ]]; then fail "El launcher $LAUNCHER es un enlace simbólico; no lo reemplacé."; fi
  if [[ -e "$LAUNCHER" ]] && ! is_managed_file "$LAUNCHER" CHXCHX_STUDIO_LAUNCHER CHXCHX_STUDIO_DEV_LAUNCHER; then
    fail "Ya existe $LAUNCHER y no está administrado por ChxChx; no lo sobrescribí."
  fi
fi
if [[ "$UPDATE_ONLY" == true ]]; then
  if [[ "$CLI_ONLY" == true ]]; then
    is_managed_file "$SHIM" CHXCHX_CLI_SHIM || fail "--update requiere una instalación previa del comando chxchx-tech administrada por ChxChx."
  else
    is_managed_file "$LAUNCHER" CHXCHX_STUDIO_LAUNCHER CHXCHX_STUDIO_DEV_LAUNCHER \
      || fail "--update requiere una instalación previa con launcher administrado en $LAUNCHER."
  fi
fi
if [[ -n "$APP_LINK" && ( -e "$APP_LINK" || -L "$APP_LINK" ) ]]; then
  [[ -L "$APP_LINK" ]] || fail "Ya existe $APP_LINK y no es el enlace administrado por ChxChx."
  LINK_TARGET="$(readlink "$APP_LINK")"
  case "$LINK_TARGET" in
    "$MAC_APPS"/ChxChx\ Studio*-*.app) ;;
    *) fail "El enlace $APP_LINK apunta a una ruta ajena; no lo cambié." ;;
  esac
  [[ -f "$LINK_TARGET/.chxchx-managed" ]] || fail "El enlace $APP_LINK no apunta a una app administrada por ChxChx."
fi
NEEDS_CLI_INSTALL=true
if [[ -e "$APP_ROOT" || -L "$APP_ROOT" ]]; then
  [[ ! -L "$APP_ROOT" && -d "$APP_ROOT" ]] || fail "La ruta $APP_ROOT no es un directorio administrado; no la reemplacé."
  [[ -f "$APP_ROOT/.chxchx-managed" ]] || fail "La ruta $APP_ROOT ya existe y no está administrada por ChxChx."
  EXISTING_COMMIT="$(cat "$APP_ROOT/.chxchx-managed")"
  [[ -z "$EXISTING_COMMIT" || "$EXISTING_COMMIT" == "$COMMIT_SHA" ]] || fail "El marcador de $APP_ROOT corresponde a otra versión; no la reemplacé."
  if [[ "$CLI_ONLY" == false ]]; then
    [[ -x "$APP_BINARY" ]] || fail "La instalación existente está incompleta: falta $APP_BINARY. Conservé los archivos para revisión."
  fi
  [[ -x "$CLI_BINARY" && -x "$CLI_ROOT/.venv/bin/python" ]] && NEEDS_CLI_INSTALL=false
fi

# ---------- Descarga y verificación -------------------------------------------------------------------
if [[ "$CHANNEL" == "dev" ]]; then
  printf 'Preview %s (%s)%s\n' "$MANIFEST_VERSION" "$SHORT_COMMIT" "${PLATFORM:+ · $PLATFORM}"
else
  printf 'ChxChx %s (canal estable)%s\n' "$MANIFEST_VERSION" "${PLATFORM:+ · $PLATFORM}"
fi
if [[ "$CLI_ONLY" == false ]]; then
  curl_get "${BASE}/${PACKAGE}?t=${BUSTER}" -o "$WORK_DIR/$PACKAGE"
  [[ "$(sha_file "$WORK_DIR/$PACKAGE")" == "$PACKAGE_SHA" ]] || fail "El SHA-256 de Studio no coincide; no se instaló nada."
fi
curl_get "${BASE}/${WHEEL_NAME}?t=${BUSTER}" -o "$WORK_DIR/$WHEEL_NAME"
[[ "$(sha_file "$WORK_DIR/$WHEEL_NAME")" == "$WHEEL_SHA" ]] || fail "El SHA-256 del CLI no coincide; no se instaló nada."

if [[ "$CLI_ONLY" == true ]]; then
  printf 'Comando chxchx-tech aislado: %s/.venv\n' "$APP_ROOT"
else
  printf 'Instalación de Studio: %s\nCLI bridge aislado: %s/.venv\n' "$APP_ROOT" "$CLI_ROOT"
fi
if [[ "$DRY_RUN" == true ]]; then
  printf 'DRY RUN: descargas y checksums válidos; no se instalaron paquetes ni se escribieron archivos fuera de /tmp.\n'
  exit 0
fi
if [[ "$NEEDS_CLI_INSTALL" == false ]] && ! "$CLI_BINARY" --help >/dev/null 2>&1; then
  NEEDS_CLI_INSTALL=true
fi

# ---------- Bibliotecas del sistema (Linux) -------------------------------------------------------------
if [[ "$OS" == "Linux" && "$CLI_ONLY" == false && "$INSTALL_DEPS" == true ]]; then
  command -v sudo >/dev/null 2>&1 || fail "Instala sudo, o vuelve a ejecutar con --no-deps si ya tienes las bibliotecas gráficas de Qt."
  if [[ "$PLATFORM" == "linux-fedora-x86_64" ]]; then
    command -v dnf >/dev/null 2>&1 || fail "No se encontró dnf."
    sudo dnf install -y fontconfig libglvnd-egl libxkbcommon-x11 libX11-xcb libxcb \
      mesa-libGL xcb-util xcb-util-cursor xcb-util-image xcb-util-keysyms \
      xcb-util-renderutil xcb-util-wm
  else
    command -v apt-get >/dev/null 2>&1 || fail "No se encontró apt-get."
    sudo apt-get update
    sudo apt-get install -y fontconfig libegl1 libgl1 libx11-xcb1 libxkbcommon-x11-0 \
      libxcb-cursor0 libxcb-icccm4 libxcb-image0 libxcb-keysyms1 libxcb-randr0 \
      libxcb-glx0 libxcb-render0 libxcb-render-util0 libxcb-shape0 libxcb-shm0 \
      libxcb-sync1 libxcb-util1 libxcb-xfixes0 libxcb-xinerama0 libxcb-xkb1
  fi
fi

# ---------- Instalación del paquete -----------------------------------------------------------------------
write_marker() { # carpeta
  printf '%s\n' "$COMMIT_SHA" > "$1/.chxchx-managed.tmp.$$"
  mv -f "$1/.chxchx-managed.tmp.$$" "$1/.chxchx-managed"
  printf 'channel=%s\nversion=%s\ncommit=%s\n' "$CHANNEL" "$MANIFEST_VERSION" "$COMMIT_SHA" > "$1/.chxchx-install"
}
mkdir -p "$APP_PARENT"
if [[ "$CLI_ONLY" == true ]]; then
  mkdir -p "$APP_ROOT"
  write_marker "$APP_ROOT"
elif [[ -d "$APP_ROOT" ]]; then
  write_marker "$APP_ROOT"
else
  STAGE="$(mktemp -d "$APP_PARENT/.chxchx-stage.XXXXXX")"
  if [[ "$OS" == "Darwin" ]]; then
    MOUNT_DIR="$WORK_DIR/mount"
    mkdir -p "$MOUNT_DIR"
    hdiutil attach "$WORK_DIR/$PACKAGE" -readonly -nobrowse -mountpoint "$MOUNT_DIR" >/dev/null
    APP_BUNDLE="$(find "$MOUNT_DIR" -type d -name '*.app' -prune -print -quit)"
    [[ -n "$APP_BUNDLE" ]] || fail "No se encontró la app dentro del DMG."
    STAGED_APP="$STAGE/${APP_LABEL}-${SUFFIX}.app"
    ditto "$APP_BUNDLE" "$STAGED_APP"
    hdiutil detach "$MOUNT_DIR" >/dev/null
    MOUNT_DIR=""
    [[ -x "$STAGED_APP/Contents/MacOS/chxchx-studio" ]] || fail "El paquete macOS no contiene el ejecutable esperado."
    write_marker "$STAGED_APP"
    mv "$STAGED_APP" "$APP_ROOT"
  else
    tar -xzf "$WORK_DIR/$PACKAGE" -C "$STAGE" --strip-components=1
    [[ -x "$STAGE/bin/chxchx-studio" ]] || fail "El paquete Linux no contiene bin/chxchx-studio."
    write_marker "$STAGE"
    mv "$STAGE" "$APP_ROOT"
  fi
  rmdir "$STAGE" 2>/dev/null || true
  STAGE=""
fi

# ---------- Comando chxchx-tech (entorno aislado con uv) ----------------------------------------------------
if [[ "$NEEDS_CLI_INSTALL" == true ]]; then
  if ! command -v uv >/dev/null 2>&1 && [[ ! -x "$HOME/.local/bin/uv" ]]; then
    curl --fail --silent --show-error --location --proto '=https' https://astral.sh/uv/install.sh | sh
  fi
  UV_BIN="$(command -v uv || true)"
  [[ -n "$UV_BIN" ]] || UV_BIN="$HOME/.local/bin/uv"
  [[ -x "$UV_BIN" ]] || fail "No se encontró uv después de instalarlo."
  if [[ ! -x "$CLI_ROOT/.venv/bin/python" || ! -x "$CLI_BINARY" ]]; then
    "$UV_BIN" venv --clear --python 3.11 "$CLI_ROOT/.venv"
  fi
  "$UV_BIN" pip install --python "$CLI_ROOT/.venv/bin/python" \
    --reinstall-package chxchx-tech-lead "$WORK_DIR/$WHEEL_NAME"
  [[ -x "$CLI_BINARY" ]] || fail "El wheel no dejó el comando chxchx-tech esperado."
  "$CLI_BINARY" --help >/dev/null || fail "El bridge CLI instalado no puede iniciarse."
fi
[[ -z "$APP_LINK" ]] || ln -sfn "$APP_ROOT" "$APP_LINK"

# ---------- Launcher, comando y menú de aplicaciones ------------------------------------------------------------
mkdir -p "$LOCAL_BIN"
write_script() { # destino contenido
  local tmp="$1.tmp.$$"
  printf '%s\n' "$2" > "$tmp"
  chmod 755 "$tmp"
  mv "$tmp" "$1"
}

LAUNCHER_MARKS="# CHXCHX_STUDIO_LAUNCHER channel=${CHANNEL}"
# El preview anterior marcaba así sus launchers: se conserva para reconocerlos al actualizar.
if [[ "$CHANNEL" == "dev" ]]; then LAUNCHER_MARKS="${LAUNCHER_MARKS}"$'\n'"# CHXCHX_STUDIO_DEV_LAUNCHER"; fi
if [[ "$CLI_ONLY" == false ]]; then
  if [[ "$OS" == "Darwin" ]]; then
    write_script "$LAUNCHER" "#!/usr/bin/env bash
${LAUNCHER_MARKS}
APP_ROOT=\"${APP_LINK}\"
export PATH=\"\$HOME/.local/bin:\$PATH\"
export CHXCHX_TECH_CLI=\"\$APP_ROOT/Contents/.venv/bin/chxchx-tech\"
exec \"\$APP_ROOT/Contents/MacOS/chxchx-studio\" \"\$@\""
  else
    write_script "$LAUNCHER" "#!/usr/bin/env bash
${LAUNCHER_MARKS}
APP_ROOT=\"${APP_ROOT}\"
export PATH=\"\$HOME/.local/bin:\$PATH\"
export CHXCHX_TECH_CLI=\"\$APP_ROOT/.venv/bin/chxchx-tech\"
exec \"\$APP_ROOT/bin/chxchx-studio\" \"\$@\""
  fi
fi

SHIM_NOTE=""
if [[ -L "$SHIM" ]] || { [[ -e "$SHIM" ]] && ! is_managed_file "$SHIM" CHXCHX_CLI_SHIM; }; then
  SHIM_NOTE="Ya existe $SHIM y no es de ChxChx (quizá instalado con uv); no lo reemplacé. El comando de esta instalación está en $CLI_BINARY"
else
  write_script "$SHIM" "#!/usr/bin/env bash
# CHXCHX_CLI_SHIM
exec \"${CLI_BINARY}\" \"\$@\""
fi

if [[ "$OS" == "Linux" && "$CLI_ONLY" == false ]]; then
  DESKTOP="$DATA_HOME/applications/${LAUNCHER_NAME}.desktop"
  ICON_SOURCE="$APP_ROOT/share/icons/hicolor/256x256/apps/chxchx-studio.png"
  if [[ -e "$DESKTOP" ]] && ! is_managed_file "$DESKTOP" CHXCHX_STUDIO_DESKTOP; then
    warn "Ya existe $DESKTOP y no es de ChxChx; no creé la entrada del menú."
  else
    mkdir -p "$DATA_HOME/applications"
    if [[ -f "$ICON_SOURCE" ]]; then
      mkdir -p "$DATA_HOME/icons/hicolor/256x256/apps"
      cp -f "$ICON_SOURCE" "$DATA_HOME/icons/hicolor/256x256/apps/chxchx-studio.png"
    fi
    cat > "$DESKTOP" <<DESKTOP_ENTRY
[Desktop Entry]
# CHXCHX_STUDIO_DESKTOP
Type=Application
Name=${APP_LABEL}
Comment=Editor y panel de control para proyectos con agentes
Exec="${LAUNCHER}" %f
Icon=chxchx-studio
Terminal=false
Categories=Development;IDE;
StartupWMClass=chxchx-studio
MimeType=inode/directory;
DESKTOP_ENTRY
    command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database "$DATA_HOME/applications" >/dev/null 2>&1 || true
  fi
fi

# ---------- Limpieza: se conservan la versión nueva y la anterior (para volver atrás) -----------------------------
touch "$APP_ROOT"
KEPT=0
if [[ "$OS" == "Darwin" && "$CLI_ONLY" == false ]]; then CANDIDATES=("$APP_PARENT/${APP_LABEL}"-*.app); else CANDIDATES=("$APP_PARENT/${CHANNEL}-"*); fi
while IFS= read -r OLD; do
  [[ -n "$OLD" ]] || continue
  KEPT=$((KEPT + 1))
  [[ "$KEPT" -gt "$KEEP_VERSIONS" ]] || continue
  [[ -d "$OLD" && ! -L "$OLD" && -f "$OLD/.chxchx-managed" ]] && rm -rf -- "$OLD"
done < <(ls -1dt "${CANDIDATES[@]}" 2>/dev/null || true)

printf '\nChxChx %s quedó instalado.\n' "$MANIFEST_VERSION"
if [[ "$CLI_ONLY" == false ]]; then
  printf 'Abre Studio con: %s /ruta/al/proyecto' "$LAUNCHER_NAME"
  [[ "$OS" == "Linux" ]] && printf '   (o desde el menú de aplicaciones)' || printf '   (o desde ~/Applications)'
  printf '\n'
fi
printf 'Comando: chxchx-tech doctor\n'
[[ -z "$SHIM_NOTE" ]] || warn "$SHIM_NOTE"
case ":$PATH:" in
  *":$LOCAL_BIN:"*) ;;
  *) printf 'Agrega ~/.local/bin a tu PATH (por ejemplo en ~/.profile) o abre una terminal nueva.\n' ;;
esac
if [[ "$OS" == "Darwin" && "$CLI_ONLY" == false ]]; then
  printf 'El preview no está firmado; macOS puede pedir confirmar Abrir una vez desde Finder.\n'
fi
printf 'Para desinstalar: install.sh --uninstall\n'
