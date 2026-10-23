# ADR 0005: Distribución pública del preview de ChxChx Studio

- Estado: Aceptada
- Fecha: 2026-10-14

## Contexto

Los artefactos de Actions no son una descarga práctica para usuarios sin acceso
al repositorio. Studio requiere además el bridge CLI Python de la misma
revisión. El objetivo es instalar desde README sin clonar el código fuente.

## Decisión

- En pushes exitosos a `dev`, CI publica un release rodante
  `studio-dev-preview` con paquetes de Ubuntu x86_64, Fedora x86_64, Windows
  x86_64, macOS Apple Silicon e Intel, además de un wheel universal del CLI.
- Como el preview reutiliza el mismo tag, CI elimina el release/tag anterior
  después de preparar y validar los nuevos assets y vuelve a publicar el tag
  apuntando al commit actual.
- `studio-preview.txt` identifica el commit, versión, paquete por plataforma y
  SHA-256. Los instaladores comprueban el manifiesto y los assets antes de
  escribir en rutas del usuario.
- El bridge CLI se instala en un `.venv` junto al paquete nativo para que Studio
  consuma la misma versión sin sustituir una instalación global.
- Los instaladores son idempotentes, rechazan destinos/launchers no
  administrados y aceptan `--dry-run` para verificar las descargas sin instalar.
  Las instalaciones se versionan para conservar una ruta clara de rollback.
- Reejecutar el instalador actualiza al commit publicado más reciente; `--update`
  (PowerShell: `-Update`) exige una instalación previa y puede combinarse con el
  modo de preview. Studio ofrece una comprobación manual desde Ayuda; solo tras
  confirmación descarga y verifica app y CLI, prepara ambos lado a lado y cambia
  el launcher al final. El marcador persistido contiene el SHA completo; el
  updater reconoce también el marcador vacío de instaladores Bash anteriores.
- La publicación actual es un preview sin firma; no se presenta como release
  estable ni promete soporte para arquitecturas sin artefacto.

## Consecuencias

- El release requiere builds, pruebas y wheel verdes en CI antes de actualizar
  los assets. Los enlaces de instalación funcionan después de esa publicación.
- Linux puede requerir bibliotecas Qt/X11 del sistema; macOS puede requerir
  autorización de primera apertura y Windows preview no tiene firma del
  publicador.
- El siguiente paso para una distribución estable es validar los paquetes en
  equipos reales y establecer firma/notarización antes de etiquetar `v1.0`.
