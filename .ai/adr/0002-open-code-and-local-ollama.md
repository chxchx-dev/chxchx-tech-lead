# ADR 0002: Agentes OpenCode y modelos locales desde Studio

- Estado: Aceptada
- Fecha: 2026-10-14

## Contexto

Studio ya inicia cualquier agente configurado por argv a través del mismo
bridge, preflight de trust/RAM y terminal PTY/ConPTY. Añadir un proveedor no
debe crear un segundo flujo de sesiones ni editar configuraciones al abrir el
proyecto.

## Decisión

- La vista Agentes permite agregar, bajo confirmación explícita, OpenCode (`opencode`)
  o OpenCode conectado a la selección guiada de modelos de Ollama
  (`ollama launch opencode`).
- El caso de uso CLI agrega entradas `[[workspace.agents]]` de forma aditiva e
  idempotente, con `--dry-run`, escritura atómica y backup previo. No modifica
  los presets declarados por el proyecto.
- El inicio y la cuota de recursos siguen pasando por los servicios y adapters
  compartidos existentes. No se descarga ni configura automáticamente un modelo:
  Ollama presenta su selector cuando se inicia su integración.
- OpenCode acepta sesiones nuevas mediante `--prompt` y puede leer la memoria
  explícita ya persistida localmente. La extracción/captura desde transcripts no
  se atribuye a OpenCode; permanece limitada a Codex y Claude.

## Consecuencias

- Se pueden habilitar agentes desde Studio sin editar TOML ni cambiar la
  configuración global de OpenCode/Ollama.
- El hash local de confianza incluye el TOML: después de agregar un agente,
  el usuario debe volver a dar trust al proyecto antes de iniciar sesiones.
- La integración local necesita que el usuario instale Ollama y elija un modelo
  acorde a su equipo. No se promete una detección automática de GPU/RAM ni un
  nivel de capacidad del modelo.
- Chats e historial de transcripciones continúan limitados a los proveedores
  que ya tienen adapters de historial.
