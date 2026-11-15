from __future__ import annotations

from pathlib import Path

from .models import ProjectInfo
from .project_config import memory_project_name
from .profiles import profile_rules


DOC_DIRS = [
    ".ai/memory",
    "docs/architecture",
    "docs/adr",
    "docs/security",
    "docs/database",
    "docs/api",
]


def project_rule_body(info: ProjectInfo) -> str:
    stacks = ", ".join(info.stacks)
    languages = ", ".join(info.languages) or "no detectados"
    memory = memory_project_name(info)
    extra_rules = "\n".join(f"- {rule}" for rule in profile_rules(info.profile_name))
    profile_section = f"\n## Reglas del perfil\n\n{extra_rules}\n" if extra_rules else ""
    return f"""# Reglas administradas por chxchx-tech-lead

Proyecto: **{info.name}**
Perfil detectado: **{info.profile_name}**
Stacks: {stacks}
Lenguajes: {languages}
Basic Memory project: `{memory}`

## Protocolo de trabajo

1. Lee `.ai/PROJECT.md` y `.ai/CURRENT_STATE.md` antes de cambios amplios.
2. Consulta `docs/adr/` y Basic Memory antes de contradecir decisiones existentes.
3. Haz cambios pequeños, verificables y con pruebas cuando corresponda.
4. Valida el resultado y deja `.ai/HANDOFF.md` actualizado si queda trabajo incompleto.
5. Antes de dar por terminada una tarea con cambios, decisiones o hallazgos útiles, guarda un checkpoint sin pedirle al usuario que lo haga: actualiza `.ai/CURRENT_STATE.md` y `.ai/HANDOFF.md`, y usa la herramienta `write_memory` de Basic Memory para decisiones y conocimiento reutilizable de este proyecto.

## Reglas

- No introduzcas secretos en código, documentación, logs o commits.
- Si el usuario pide explícitamente recordar o conservar algo para futuras sesiones, añádelo de inmediato como entrada fechada en `.ai/memory/PROJECT_MEMORY.md`; conserva literalmente frases o nombres, no reemplaces entradas existentes y confirma la ruta guardada.
- Al iniciar una tarea o conversación nueva, consulta `.ai/memory/PROJECT_MEMORY.md` si existe y recupera solo las entradas pertinentes al proyecto y la solicitud.
- Si el usuario pregunta por una memoria explícita que no aparece allí, busca únicamente la petición correspondiente en los chats recientes de este mismo proyecto; si la recuperas, persístela ahora. Si no aparece, di que no quedó guardada y pídele el dato otra vez.
- No guardes contraseñas, tokens, claves, datos personales sensibles ni transcripciones completas. No conviertas comentarios casuales en memoria persistente.
- Si la tarea depende de decisiones anteriores, consulta Basic Memory usando el proyecto `{memory}`.
- Al llamar herramientas de Basic Memory, usa exactamente `{memory}` en el argumento `project`; no reutilices un `project_id` de otro proyecto ni el ámbito predeterminado. Si no puedes fijar el ámbito correcto, usa los archivos locales y no llames Basic Memory.
- No asumas que Basic Memory conserva transcripciones: guarda allí solo conocimiento duradero, y no mezcles información de otros proyectos.
- No guardes saludos, preguntas triviales, secretos ni transcripciones completas; si no hubo cambios ni conocimiento reutilizable, no crees una nota vacía.
- Usa Serena para navegación semántica del código cuando esté disponible.
{profile_section}"""


def claude_body() -> str:
    return """# Claude specific

@AGENTS.md

- Usa Basic Memory para recuperar decisiones anteriores cuando la tarea dependa de contexto persistente.
- Antes de responder al terminar trabajo sustancial, persiste un checkpoint sin pedir una acción manual: usa `write_memory` para decisiones reutilizables y actualiza `.ai/CURRENT_STATE.md` / `.ai/HANDOFF.md` cuando reflejen el estado o pendientes actuales.
- No guardes saludos, preguntas triviales, secretos ni transcripciones completas; si no hubo cambio o conocimiento reutilizable, no crees una nota.
- Usa Serena para explorar símbolos y referencias antes de hacer búsquedas masivas por texto.
"""


def ai_files(info: ProjectInfo) -> dict[str, str]:
    return {
        ".ai/PROJECT.md": f"""# {info.name}\n\n## Propósito\n\nDescribe aquí qué problema resuelve este proyecto.\n\n## Stack detectado\n\n- Perfil: `{info.profile_name}`\n- Stacks: {', '.join(info.stacks)}\n- Lenguajes: {', '.join(info.languages) or 'pendiente'}\n\n## Restricciones\n\n- Añadir restricciones técnicas y de negocio importantes.\n""",
        ".ai/CURRENT_STATE.md": """# Current State\n\n## Estado actual\n\n- Inicializado con chxchx-tech-lead.\n\n## En progreso\n\n- Pendiente de completar.\n\n## Bloqueos\n\n- Ninguno registrado.\n""",
        ".ai/ROADMAP.md": """# Roadmap\n\n## Ahora\n\n- Define el siguiente entregable verificable.\n\n## Después\n\n- Pendiente.\n""",
        ".ai/HANDOFF.md": """# Handoff\n\n## Último cambio\n\n- Sin handoff todavía.\n\n## Pendiente\n\n- Ninguno.\n\n## Validación\n\n- Añadir comandos de prueba/build relevantes.\n""",
    }


def create_project_structure(
    info: ProjectInfo,
    dry_run: bool = False,
    *,
    minimal: bool = False,
) -> list[str]:
    actions: list[str] = []
    directories = [".ai/memory"] if minimal else DOC_DIRS
    files = {} if minimal else ai_files(info)
    for rel in directories:
        path = info.root / rel
        if not path.exists():
            actions.append(f"create dir {rel}")
            if not dry_run:
                path.mkdir(parents=True, exist_ok=True)

    for rel, content in files.items():
        path = info.root / rel
        if not path.exists():
            actions.append(f"create {rel}")
            if not dry_run:
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(content, encoding="utf-8")
    return actions
