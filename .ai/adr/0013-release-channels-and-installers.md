# ADR-0013: Canales de release e instaladores por sistema

- Estado: Aceptada (Windows, macOS, NSIS y publicación real pendientes de validar en CI)
- Fecha: 2026-11-03
- Relacionada: ADR 0005 (distribución del preview)

## Contexto

El preview se instalaba con scripts distintos por sistema (`install-studio.sh`,
`install-studio.ps1`, `Setup.exe`), solo existía el canal rodante `studio-dev-preview` y el
manifiesto se armaba con un bloque embebido en el CI. No había versión estable, ni desinstalador,
ni forma de probar los instaladores sin publicar.

## Decisión

- **Dos canales.** `stable` (tag `vX.Y.Z`; el tag debe coincidir con `pyproject.toml` y no llevar `.dev`)
  y `dev` (`studio-dev-preview`, reconstruido desde la rama `dev`). Conviven en carpetas distintas
  (`stable-X.Y.Z`, `dev-<commit12>`; en macOS `ChxChx Studio[ Dev]-<sufijo>.app`) y con launchers distintos
  (`chxchx-studio`, `chxchx-studio-dev`).
- **Un manifiesto, un generador.** `scripts/release_assets.py` copia los paquetes, escribe
  `studio-manifest.txt` (esquema 1: canal, tag, commit, versión, `asset.*`, `sha256.*`) y `SHA256SUMS`;
  el canal dev publica además `studio-preview.txt` para instaladores antiguos. El CI usa solo este script.
- **Un instalador por familia** (`install.sh` para Linux/macOS, `install.ps1` para Windows), publicados
  como assets del release y accesibles por `releases/latest/download/…`. Opciones comunes: canal,
  versión, `--update`, `--cli-only`, `--uninstall`, `--dry-run`. Verifican el SHA-256 de cada descarga
  contra el manifiesto antes de instalar, instalan en el perfil del usuario (sin administrador), usan
  carpetas por versión con marcador `.chxchx-managed`, conservan la versión anterior y **nunca
  sobrescriben rutas, launchers, accesos o shims que no lleven su marcador**. Desinstalar quita solo lo
  administrado y no toca `~/.chxchx-tech-lead` ni `.ai/`.
- **Linux**: TGZ + entrada `.desktop` + icono; familias por `ID`/`ID_LIKE` (Debian/Ubuntu, Fedora/RHEL);
  se comprueba glibc ≥ 2.35. **Windows**: ZIP, accesos en el menú Inicio, entrada en *Aplicaciones
  instaladas* (HKCU), `PATH` de usuario editado en el registro. `Setup.exe` (NSIS) solo empaqueta `install.ps1`.
- **Origen de pruebas.** `CHXCHX_TEST_BASE` redirige las descargas, pero solo a `http://127.0.0.1` o
  `localhost`. El CI (`installer-smoke`) sirve el release preparado desde localhost y ejecuta
  instalar → lanzar `--version` → actualizar → desinstalar en Ubuntu, macOS (ambas arquitecturas) y Windows.
- **Actualizador de Studio** (Python) sigue el canal de la instalación en curso y reconoce ambos nombres de
  carpeta; reescribe el launcher y el shim `chxchx-tech` solo si son de ChxChx.
- **Versión única.** `pyproject.toml` manda: CMake la lee y Studio responde `--version` sin abrir ventana.
- **Firma preparada, desactivada.** `scripts/sign-windows.ps1` (Authenticode) y `scripts/sign-macos.sh`
  (Developer ID, notarytool, staple) se ejecutan en CI solo si existen los secretos; hasta entonces los
  paquetes salen sin firmar y el README lo dice.

## Consecuencias

- Publicar una versión estable es empujar un tag `vX.Y.Z` con la versión ya sin `.dev`.
- El SHA-256 del manifiesto protege contra descargas corruptas, no contra quien controle el release: la firma
  de código es el siguiente paso cuando haya certificados.
- Los scripts de Windows y los flujos de macOS no se pudieron ejecutar localmente; hasta que pase
  `installer-smoke` en CI se consideran sin verificar.
