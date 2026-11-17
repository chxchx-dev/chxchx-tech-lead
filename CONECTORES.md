# Conectores MCP: dale manos a tus agentes en tus apps

Esta guía explica cómo conectar a Claude Code, Codex y OpenCode con **Excel, Power BI, Word, Google Sheets,
PostgreSQL, SQLite** o con cualquier otro servidor MCP, desde ChxChx Studio, el TUI o la terminal.

## 1. Qué es y para qué sirve

**MCP** (Model Context Protocol) es el estándar con el que un agente de IA usa herramientas externas. Un **conector** es un
servidor MCP que sabe trabajar con una app o un tipo de dato: con el de Excel tu agente puede leer y editar un `.xlsx`; con el
de PostgreSQL, consultar una base; con el de Power BI, modelar medidas DAX.

ChxChx no es el servidor ni el agente: es quien **lo deja configurado bien**, en el proyecto y para cada agente, enseñándote
primero exactamente qué va a escribir y sin guardar nunca una clave.

## 2. Antes de empezar

| Necesitas | Para qué | Cómo comprobarlo |
|---|---|---|
| Un agente instalado (Claude Code, Codex u OpenCode) | Es quien usará el conector | Studio → panel *Agentes* |
| Un proyecto abierto y preparado | Los conectores se guardan en el proyecto | `chxchx-tech status .` |
| `npx` (viene con [Node.js](https://nodejs.org)) | Excel y Power BI | `npx --version` |
| `uvx` (viene con [uv](https://docs.astral.sh/uv/)) | Word, Google Sheets, PostgreSQL y SQLite | `uvx --version` |

Cada ficha del conector marca con ✓/✗ si lo que necesita está en tu equipo. Sin ello puedes **añadirlo igual**, pero el agente
no podrá iniciar el servidor hasta que lo instales.

## 3. Inicio rápido

### En Studio
1. Abre **Herramientas / MCP** (barra de herramientas «Conectar» o el menú *Ir*).
2. Baja a **Conectores MCP**. Cada tarjeta muestra qué hace, qué necesita y, en chips, en cuáles de tus agentes ya está
   (`✓` configurado, `≠` existe con otra configuración, `·` sin configurar).
3. Pulsa **Añadir**. Si el conector pide un valor (la ruta de una base SQLite), escríbelo antes en la propia tarjeta.
4. Revisa la **vista previa**: lista lo que cambiará, los avisos y el contenido exacto de cada archivo.
5. Pulsa **Aplicar**. Reinicia el agente (o abre una sesión nueva) para que cargue el servidor.

Para quitarlo, la misma tarjeta muestra **Quitar** cuando ya está configurado.

### En el TUI
`chxchx-tech tui .` → página **6 Conectar**. Elige el conector con ↑↓, `Intro` abre la vista previa y `y` la aplica. Si
falta un valor te lo pide en un cuadro. `Esc` cancela sin cambiar nada.

### En la terminal
```bash
chxchx-tech connectors list                      # qué hay y dónde está configurado
chxchx-tech connectors add excel word            # vista previa (no escribe)
chxchx-tech connectors add excel word --show     # …con el contenido exacto de cada archivo
chxchx-tech connectors add excel word --apply    # lo escribe, con respaldo
chxchx-tech connectors remove excel --apply      # lo quita (solo lo que añadió ChxChx)
```
`conn` es el atajo de `connectors`. Con `--client claude|codex|opencode|all` eliges en qué agentes; sin él se usan los que
tienes instalados o que ya tienen su archivo en el proyecto. `--path RUTA` apunta a otro proyecto.

## 4. Los conectores, uno por uno

| Conector | Qué hace | Necesita | Origen |
|---|---|---|---|
| `excel` | Leer y editar libros `.xlsx` | `npx` | Comunidad |
| `powerbi` | Modelar en Power BI (tablas, medidas DAX, relaciones) | `npx` | Microsoft (vista previa) |
| `word` | Crear y editar `.docx` | `uvx` | Comunidad |
| `google-sheets` | Hojas de cálculo de Google | `uvx` + cuenta de servicio | Comunidad |
| `postgres` | Consultar y analizar PostgreSQL | `uvx` + `DATABASE_URI` | Comunidad |
| `sqlite` | Consultar y analizar un archivo SQLite | `uvx` + ruta del archivo | Referencia MCP |

### Excel
- **Qué hace:** lee hojas, celdas y fórmulas; escribe valores y tablas; crea gráficos. No necesita tener Excel abierto.
- **En Windows**, además, puede editar en vivo el libro abierto; en macOS y Linux trabaja sobre el archivo.
- **Variable opcional:** `EXCEL_MCP_PAGING_CELLS_LIMIT` (celdas por lectura, 4000 por defecto). No se escribe en el proyecto.
- **Pídele al agente:** «Abre `ventas.xlsx`, resume la hoja *2026* y dime qué meses cayeron frente al anterior» · «Añade una columna
  *Margen %* con su fórmula y una tabla dinámica por región».

### Power BI (modelado)
- **Qué hace:** el servidor de Microsoft para el *modelado*: tablas, columnas, medidas DAX, relaciones y jerarquías de un modelo
  semántico. No edita páginas de informes ni su diseño.
- **Importante:** está en **vista previa pública** y puede cambiar; pide aceptar su EULA la primera vez. El comando viene de
  los listados del proyecto: **comprueba que coincide con su repositorio**
  (<https://github.com/microsoft/powerbi-modeling-mcp>) antes de confiar en él. La vista previa te lo muestra completo.
- **Pídele al agente:** «Revisa el modelo y propón una medida de margen sobre ventas» · «Documenta las relaciones y señala las ambiguas».

### Word
- **Qué hace:** crea y edita `.docx` (párrafos, estilos, tablas, imágenes, comentarios). No necesita Word instalado.
- **Pídele al agente:** «Genera un informe en `informe.docx` con estos resultados y una tabla resumen».

### Google Sheets
1. Crea una **cuenta de servicio** en Google Cloud y descarga su clave JSON.
2. **Comparte** tu carpeta de Drive con el correo de esa cuenta (rol *Editor*).
3. Define `SERVICE_ACCOUNT_PATH` (ruta a la clave) y, si quieres acotar, `DRIVE_FOLDER_ID` en tu entorno (sección 5).
- **Pídele al agente:** «Lista las hojas de la carpeta y copia los totales de *Ventas* a *Resumen*».

### PostgreSQL
- **Qué hace:** esquemas, consultas SQL, planes de ejecución y salud de índices.
- **Seguro por defecto:** arranca en **modo restringido (solo lectura)**. Para permitir escritura habría que cambiar el comando a mano
  en el archivo del agente (`--access-mode=unrestricted`); piénsalo dos veces con bases reales.
- **Necesita** `DATABASE_URI`, por ejemplo `postgresql://usuario:clave@host:5432/base`. Lleva la contraseña: va en tu
  entorno, nunca en el proyecto.
- **Pídele al agente:** «Explícame el esquema de pedidos y encuentra las consultas más lentas».

### SQLite
- **Qué hace:** tablas, esquemas y consultas sobre un archivo `.db`/`.sqlite`.
- **Valor requerido:** la ruta del archivo (en Studio, el campo de la tarjeta; en la terminal,
  `--param db_path=./datos.db`). Ese valor sí se guarda en el archivo de configuración: no pongas rutas privadas si compartes el repo.
- **Con permiso de lectura y escritura** sobre ese archivo: trabaja con una copia si importa.

## 5. Claves y variables de entorno

Un conector que necesita una clave (`DATABASE_URI`, `SERVICE_ACCOUNT_PATH`) **no la recibe de ChxChx**. En el archivo del agente
solo queda el *nombre* de la variable, y el agente la lee de tu entorno al iniciar el servidor:

| Agente | Cómo queda escrito | Archivo |
|---|---|---|
| Claude Code | `"env": {"DATABASE_URI": "${DATABASE_URI}"}` | `.mcp.json` |
| OpenCode | `"environment": {"DATABASE_URI": "{env:DATABASE_URI}"}` | `opencode.json` |
| Codex | `env_vars = ["DATABASE_URI"]` | `.codex/config.toml` |

Tienes que **definir la variable antes de abrir el agente**, en la misma sesión desde la que lo lanzas:

```bash
# Linux / macOS (bash o zsh); añádelo a ~/.bashrc o ~/.zshrc para que sea permanente
export DATABASE_URI='postgresql://usuario:clave@host:5432/base'
```
```powershell
# Windows (PowerShell); para dejarla fija: [Environment]::SetEnvironmentVariable('DATABASE_URI', '…', 'User')
$env:DATABASE_URI = 'postgresql://usuario:clave@host:5432/base'
```

Si abres Studio desde el menú del sistema, hereda el entorno de tu sesión de usuario (no el de una terminal que abriste después):
define la variable de forma permanente o abre Studio con `chxchx-tech studio .` desde la terminal donde la definiste.
La vista previa te avisa cuando una variable obligatoria no está definida.

## 6. Qué archivos se tocan

| Agente | Archivo del proyecto | ¿Se comparte con el equipo? |
|---|---|---|
| Claude Code | `.mcp.json` | Sí, es el archivo de equipo de Claude Code |
| Codex | `.codex/config.toml` | Depende de si lo versionas |
| OpenCode | `opencode.json` | Sí, junto a tu configuración de OpenCode |

- Solo se añade o quita la entrada del conector; **el resto del archivo queda como estaba**, en el mismo orden.
- Si el archivo ya tiene un servidor con **el mismo nombre pero otra configuración**, no se pisa: la vista previa lo avisa y lo deja.
- Si el archivo no es JSON/TOML válido, se informa y **no se modifica**.
- Como son archivos de proyecto, un compañero que los reciba usará los mismos conectores. En **Windows** los conectores con `npx`
  se escriben como `cmd /c npx …` (lo que Windows necesita); en macOS/Linux ese comando no sirve, y la vista previa te lo advierte.

## 7. Seguridad y límites

- **Siempre ves antes de cambiar.** Nada se escribe sin vista previa y confirmación (`--apply`, botón *Aplicar* o `y`).
- **Respaldo de lo que existía.** Antes de modificar un archivo se guarda una copia en
  `~/.chxchx-tech-lead/backups-mcp/<proyecto>/<fecha>/`. Se guarda fuera del proyecto a propósito.
- **Quitar es seguro.** `remove` solo borra lo que coincide con lo que escribió ChxChx; lo que editaste tú se respeta.
- **Los servidores corren en tu equipo con tus permisos** cuando el agente los inicia. Un servidor MCP puede leer y escribir lo que
  su app permita: elige conectores de origen conocido, mira el comando y trata su contenido como código de terceros.
- **El origen se indica**: *oficial* (de quien hace la app), *comunidad* (proyecto independiente) o *referencia* (ejemplo del propio MCP).
- Configurar un conector **no inicia nada**: lo inicia el agente cuando lo necesita.

## 8. Quitar y deshacer

```bash
chxchx-tech connectors remove excel --apply     # lo quita de los agentes elegidos
```
En Studio y en el TUI, el mismo botón/tecla pasa a *Quitar* cuando el conector está configurado. Si quieres volver a un archivo
tal como estaba, está en la carpeta de respaldo de la sección 7. (`chxchx-tech rollback` **no** los restaura: rehace los archivos
de preparación del proyecto, no los de MCP.)

## 9. Un servidor MCP propio

Para uno que no está en el catálogo:

```bash
chxchx-tech connectors custom mi-api --command npx --arg -y --arg mi-paquete --env MI_TOKEN --apply
```
- `--command` es el programa y cada `--arg` un argumento (repítelo). `--env NOMBRE` declara una variable que el servidor lee de tu entorno.
- El nombre admite letras, números, `-` y `_`. En Studio está la tarjeta **Servidor MCP propio** con los mismos campos
  (el comando se separa por espacios; las variables, por comas).

## 10. Si algo no funciona

| Síntoma | Causa probable | Qué hacer |
|---|---|---|
| La tarjeta marca `✗ npx` o `✗ uvx` | Falta el programa | Instala [Node.js](https://nodejs.org) o [uv](https://docs.astral.sh/uv/) y reabre ChxChx |
| El agente no ve el servidor | Aún no recargó su configuración | Cierra y vuelve a abrir el agente o inicia una sesión nueva. Puedes comprobar con `claude mcp list` u `opencode mcp list` |
| El servidor falla al arrancar con un error de variable | No definiste la variable de entorno | Defínela (sección 5) y reabre el agente desde la misma sesión |
| «ya existe con otra configuración» | Hay un servidor con ese nombre que no escribió ChxChx | Revísalo en el archivo; bórralo a mano si quieres que ChxChx lo reemplace |
| «no es JSON válido; no se modifica» | El archivo tiene un error de sintaxis | Corrígelo y repite; ChxChx no toca archivos rotos |
| «No hay agentes donde configurar» | No hay ninguno instalado ni con archivo en el proyecto | Instala uno o elige `--client claude\|codex\|opencode` |
| Con Codex no aparece | Codex solo carga `.codex/config.toml` de proyectos de confianza (comprobado) | Marca el proyecto como de confianza en Codex |
| Claude Code no usa el servidor | Pide aprobar los servidores de `.mcp.json` la primera vez (comprobado) | Apruébalo al abrir el proyecto, o `claude mcp reset-project-choices` |
| `opencode mcp list` no lo muestra | Probado con OpenCode 2.2.6: ni siquiera lista lo que escribe su propio `mcp add` en el proyecto | Comprueba abriendo OpenCode en el proyecto; ChxChx conserva el estilo (`mcp.<nombre>` o `mcp.servers.<nombre>`) del archivo |
| Un conector de la comunidad dejó de funcionar | El paquete cambió de nombre o argumentos | Compara con el repositorio de la ficha y actualiza el archivo; avísanos para corregir el catálogo |

## 11. Preguntas frecuentes

- **¿Puedo usarlo en todos mis proyectos a la vez?** Se configura por proyecto, a propósito: cada uno elige sus conectores y queda
  visible en su repositorio.
- **¿Funciona con Claude Desktop?** No: estos archivos son los de proyecto de Claude Code, Codex y OpenCode.
- **¿Un conector de Excel necesita Excel?** No; en Windows, si lo tienes, además puede editar el libro abierto.
- **¿Los datos salen de mi equipo?** El servidor corre localmente, pero tu **agente** puede enviar al modelo lo que lea; ten en
  cuenta la política de tu cuenta antes de conectar datos sensibles. Con modelos locales (Ollama) todo queda en tu equipo.
- **¿Qué cambia entre `integrate` y `connectors`?** `integrate` configura la memoria del proyecto (basic-memory) y el análisis de
  código (Serena); `connectors` añade apps y datos.

## 12. Para quien contribuye: añadir un conector al catálogo

Cada conector es un archivo `catalog/connectors/<nombre>.toml`, embebido en el binario al compilar:

```toml
name = "excel"                         # id: letras, números, - y _
title = "Excel"
category = "Office"                    # agrupa en las listas
description = "Lee y edita libros de Excel (.xlsx)…"
docs = "https://github.com/negokaz/excel-mcp-server"
origin = "comunidad"                   # oficial | comunidad | referencia
requires = ["npx"]                     # programa con el que se inicia (debe ser la primera palabra del comando)
command = ["npx", "--yes", "@negokaz/excel-mcp-server"]
command_windows = ["cmd", "/c", "npx", "--yes", "@negokaz/excel-mcp-server"]   # opcional
platforms = ["windows", "macos", "linux"]
note = "Aviso que se muestra en la vista previa y en la ficha."

[[env]]                                # variable que lee el servidor
name = "DATABASE_URI"                  # MAYÚSCULAS_Y_GUIONES_BAJOS
description = "URI de conexión."
secret = true                          # true: solo se nombra, jamás se escribe un valor
required = true                        # true: se declara en el archivo del agente

[[param]]                              # valor que da la persona y va en el comando como {nombre}
name = "db_path"
description = "Ruta del archivo."
placeholder = "./datos.db"
```
Las pruebas validan cada entrada (ids únicos, variables bien nombradas, que cada `{parámetro}` exista y que el comando empiece por
el programa de `requires`). **Verifica el comando contra el repositorio oficial** antes de añadirlo y, si es de terceros, dilo en
`origin` y `note`.
