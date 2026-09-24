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
    return f"""# Reglas administradas por chichan-tech-lead

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

## Reglas

- No introduzcas secretos en código, documentación, logs o commits.
- Si la tarea depende de decisiones anteriores, consulta Basic Memory usando el proyecto `{memory}`.
- Usa Serena para navegación semántica del código cuando esté disponible.
{profile_section}"""


def claude_body() -> str:
    return """# Claude specific

@AGENTS.md

- Usa Basic Memory para recuperar decisiones anteriores cuando la tarea dependa de contexto persistente.
- Usa Serena para explorar símbolos y referencias antes de hacer búsquedas masivas por texto.
"""


def ai_files(info: ProjectInfo) -> dict[str, str]:
    return {
        ".ai/PROJECT.md": f"""# {info.name}\n\n## Propósito\n\nDescribe aquí qué problema resuelve este proyecto.\n\n## Stack detectado\n\n- Perfil: `{info.profile_name}`\n- Stacks: {', '.join(info.stacks)}\n- Lenguajes: {', '.join(info.languages) or 'pendiente'}\n\n## Restricciones\n\n- Añadir restricciones técnicas y de negocio importantes.\n""",
        ".ai/CURRENT_STATE.md": """# Current State\n\n## Estado actual\n\n- Inicializado con chichan-tech-lead.\n\n## En progreso\n\n- Pendiente de completar.\n\n## Bloqueos\n\n- Ninguno registrado.\n""",
        ".ai/ROADMAP.md": """# Roadmap\n\n## Ahora\n\n- Define el siguiente entregable verificable.\n\n## Después\n\n- Pendiente.\n""",
        ".ai/HANDOFF.md": """# Handoff\n\n## Último cambio\n\n- Sin handoff todavía.\n\n## Pendiente\n\n- Ninguno.\n\n## Validación\n\n- Añadir comandos de prueba/build relevantes.\n""",
    }


def create_project_structure(info: ProjectInfo, dry_run: bool = False) -> list[str]:
    actions: list[str] = []
    for rel in DOC_DIRS:
        path = info.root / rel
        if not path.exists():
            actions.append(f"create dir {rel}")
            if not dry_run:
                path.mkdir(parents=True, exist_ok=True)

    for rel, content in ai_files(info).items():
        path = info.root / rel
        if not path.exists():
            actions.append(f"create {rel}")
            if not dry_run:
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(content, encoding="utf-8")
    return actions
