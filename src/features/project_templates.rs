use crate::features::project_config_template::memory_project_name;
use crate::features::project_detection::ProjectDetection;
use std::collections::BTreeMap;
use std::path::Path;

pub const PROJECT_DIRECTORIES: &[&str] = &[
    ".ai/memory",
    ".ai/integrations",
    "docs/architecture",
    "docs/adr",
    "docs/security",
    "docs/database",
    "docs/api",
];

pub fn initial_ai_files(name: &str, detection: &ProjectDetection) -> BTreeMap<String, String> {
    let stacks = detection.stacks.join(", ");
    let languages = if detection.languages.is_empty() {
        "pendiente".to_owned()
    } else {
        detection.languages.join(", ")
    };
    [
        (
            ".ai/PROJECT.md".to_owned(),
            format!(
                "# {name}\n\n## Propósito\n\nDescribe aquí qué problema resuelve este proyecto.\n\n## Stack detectado\n\n- Perfil: `{}`\n- Stacks: {stacks}\n- Lenguajes: {languages}\n\n## Restricciones\n\n- Añadir restricciones técnicas y de negocio importantes.\n",
                detection.profile
            ),
        ),
        (
            ".ai/CURRENT_STATE.md".to_owned(),
            "# Current State\n\n## Estado actual\n\n- Inicializado con chxchx-tech-lead.\n\n## En progreso\n\n- Pendiente de completar.\n\n## Bloqueos\n\n- Ninguno registrado.\n".to_owned(),
        ),
        (
            ".ai/ROADMAP.md".to_owned(),
            "# Roadmap\n\n## Ahora\n\n- Define el siguiente entregable verificable.\n\n## Después\n\n- Pendiente.\n".to_owned(),
        ),
        (
            ".ai/HANDOFF.md".to_owned(),
            "# Handoff\n\n## Último cambio\n\n- Sin handoff todavía.\n\n## Pendiente\n\n- Ninguno.\n\n## Validación\n\n- Añadir comandos de prueba/build relevantes.\n".to_owned(),
        ),
    ]
    .into()
}

pub fn project_rules_body(root: &Path, name: &str, detection: &ProjectDetection) -> String {
    let rules = detection
        .profile_rules
        .iter()
        .map(|rule| format!("- {rule}"))
        .collect::<Vec<_>>();
    let profile_section = if rules.is_empty() {
        String::new()
    } else {
        format!("\n## Reglas del perfil\n\n{}\n", rules.join("\n"))
    };
    let memory = memory_project_name(root, name);
    let languages = if detection.languages.is_empty() {
        "no detectados".to_owned()
    } else {
        detection.languages.join(", ")
    };
    let template = r#"# Reglas administradas por chxchx-tech-lead

Proyecto: **{{NAME}}**
Perfil detectado: **{{PROFILE}}**
Stacks: {{STACKS}}
Lenguajes: {{LANGUAGES}}
Basic Memory project: `{{MEMORY}}`

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
- Si la tarea depende de decisiones anteriores, consulta Basic Memory usando el proyecto `{{MEMORY}}`.
- Al llamar herramientas de Basic Memory, usa exactamente `{{MEMORY}}` en el argumento `project`; no reutilices un `project_id` de otro proyecto ni el ámbito predeterminado. Si no puedes fijar el ámbito correcto, usa los archivos locales y no llames Basic Memory.
- No asumas que Basic Memory conserva transcripciones: guarda allí solo conocimiento duradero, y no mezcles información de otros proyectos.
- No guardes saludos, preguntas triviales, secretos ni transcripciones completas; si no hubo cambios ni conocimiento reutilizable, no crees una nota vacía.
- Usa Serena para navegación semántica del código cuando esté disponible.
{{PROFILE_RULES}}"#;
    template
        .replace("{{NAME}}", name)
        .replace("{{PROFILE}}", &detection.profile)
        .replace("{{STACKS}}", &detection.stacks.join(", "))
        .replace("{{LANGUAGES}}", &languages)
        .replace("{{MEMORY}}", &memory)
        .replace("{{PROFILE_RULES}}", &profile_section)
}

pub fn claude_rules_body() -> &'static str {
    "# Claude specific\n\n@AGENTS.md\n\n- Usa Basic Memory para recuperar decisiones anteriores cuando la tarea dependa de contexto persistente.\n- Antes de responder al terminar trabajo sustancial, persiste un checkpoint sin pedir una acción manual: usa `write_memory` para decisiones reutilizables y actualiza `.ai/CURRENT_STATE.md` / `.ai/HANDOFF.md` cuando reflejen el estado o pendientes actuales.\n- No guardes saludos, preguntas triviales, secretos ni transcripciones completas; si no hubo cambio o conocimiento reutilizable, no crees una nota.\n- Usa Serena para explorar símbolos y referencias antes de hacer búsquedas masivas por texto.\n"
}

pub fn render_managed_block(name: &str, body: &str) -> String {
    format!(
        "<!-- chxchx-tech:start {name} -->\n{}\n<!-- chxchx-tech:end {name} -->",
        body.trim_end()
    )
}

pub fn upsert_managed_block(existing: &str, name: &str, body: &str) -> String {
    let replacement = render_managed_block(name, body);
    let start_marker = format!("<!-- chxchx-tech:start {name} -->");
    let end_marker = format!("<!-- chxchx-tech:end {name} -->");
    if let Some(start) = existing.find(&start_marker) {
        let body_start = start + start_marker.len();
        if let Some(end_offset) = existing[body_start..].find(&end_marker) {
            let end = body_start + end_offset + end_marker.len();
            return format!("{}{}{}", &existing[..start], replacement, &existing[end..]);
        }
    }
    let prefix = existing.trim_end();
    if prefix.is_empty() {
        format!("{replacement}\n")
    } else {
        format!("{prefix}\n\n{replacement}\n")
    }
}
