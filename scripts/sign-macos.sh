#!/usr/bin/env bash
# Firma (Developer ID), notariza y grapa un DMG de ChxChx Studio. CI solo lo ejecuta si existen los
# secretos listados abajo; sin ellos los releases salen sin firmar.
#
# Uso: sign-macos.sh ruta/al.dmg
# Variables: MACOS_CERT_P12_BASE64, MACOS_CERT_PASSWORD, MACOS_SIGN_IDENTITY ("Developer ID Application: ..."),
#            APPLE_ID, APPLE_TEAM_ID, APPLE_APP_PASSWORD (contraseña específica de app para notarytool).
# El certificado se importa a un llavero temporal que se elimina al terminar.
set -Eeuo pipefail

DMG="${1:?Uso: sign-macos.sh ruta/al.dmg}"
for name in MACOS_CERT_P12_BASE64 MACOS_CERT_PASSWORD MACOS_SIGN_IDENTITY APPLE_ID APPLE_TEAM_ID APPLE_APP_PASSWORD; do
  [[ -n "${!name:-}" ]] || { printf 'Falta la variable %s.\n' "$name" >&2; exit 1; }
done

WORK="$(mktemp -d)"
KEYCHAIN="$WORK/sign.keychain-db"
MOUNT="$WORK/mount"
cleanup() {
  hdiutil detach "$MOUNT" >/dev/null 2>&1 || true
  security delete-keychain "$KEYCHAIN" >/dev/null 2>&1 || true
  rm -rf "$WORK"
}
trap cleanup EXIT

KEYCHAIN_PASSWORD="$(uuidgen)"
printf '%s' "$MACOS_CERT_P12_BASE64" | base64 --decode > "$WORK/cert.p12"
security create-keychain -p "$KEYCHAIN_PASSWORD" "$KEYCHAIN"
security set-keychain-settings -lut 3600 "$KEYCHAIN"
security unlock-keychain -p "$KEYCHAIN_PASSWORD" "$KEYCHAIN"
security import "$WORK/cert.p12" -k "$KEYCHAIN" -P "$MACOS_CERT_PASSWORD" -T /usr/bin/codesign
security set-key-partition-list -S apple-tool:,apple: -s -k "$KEYCHAIN_PASSWORD" "$KEYCHAIN" >/dev/null
security list-keychains -d user -s "$KEYCHAIN" $(security list-keychains -d user | tr -d '"')

mkdir "$MOUNT"
hdiutil attach "$DMG" -readonly -nobrowse -mountpoint "$MOUNT" >/dev/null
APP_NAME="$(basename "$(find "$MOUNT" -maxdepth 1 -type d -name '*.app' -print -quit)")"
[[ -n "$APP_NAME" ]] || { echo 'El DMG no contiene una .app.' >&2; exit 1; }
mkdir "$WORK/dmg-root"
ditto "$MOUNT/$APP_NAME" "$WORK/dmg-root/$APP_NAME"
hdiutil detach "$MOUNT" >/dev/null

codesign --force --deep --options runtime --timestamp --sign "$MACOS_SIGN_IDENTITY" --keychain "$KEYCHAIN" "$WORK/dmg-root/$APP_NAME"
codesign --verify --deep --strict "$WORK/dmg-root/$APP_NAME"
ln -s /Applications "$WORK/dmg-root/Applications"
rm -f "$DMG"
hdiutil create -volname "ChxChx Studio" -srcfolder "$WORK/dmg-root" -ov -format UDZO "$DMG" >/dev/null
codesign --force --timestamp --sign "$MACOS_SIGN_IDENTITY" --keychain "$KEYCHAIN" "$DMG"

xcrun notarytool submit "$DMG" --apple-id "$APPLE_ID" --team-id "$APPLE_TEAM_ID" --password "$APPLE_APP_PASSWORD" --wait
xcrun stapler staple "$DMG"
xcrun stapler validate "$DMG"
printf 'DMG firmado y notarizado: %s\n' "$DMG"
