#pragma once

struct Area {
    const char *id;
    const char *label;
    const char *description;
};

inline constexpr Area areas[] = {
    {"overview", "Inicio", "Estado del proyecto y accesos rápidos."},
    {"project", "Proyecto", "Procesos, confianza y terminales del workspace."},
    {"agents", "Agentes", "Disponibilidad, sesiones, presets y chats."},
    {"processes", "Procesos", "Procesos administrados y consumo."},
    {"projects", "Proyectos", "Proyectos registrados y cambio de contexto."},
    {"skills", "Skills", "Catálogo, recomendaciones y selección por proyecto."},
    {"packs", "Tech Packs", "Packs detectados y composición de skills."},
    {"resources", "Recursos", "RAM, swap, CPU y procesos por proyecto."},
    {"handoff", "Handoff", "Estado transferible entre sesiones de trabajo."},
    {"memory", "Memoria", "Notas persistentes de este proyecto."},
    {"chats", "Chats", "Historial local de conversaciones de agentes."},
    {"errors", "Errores", "Errores recientes registrados por ChxChx."},
    {"setup", "Configuración", "Init, herramientas, MCP y diagnóstico."},
    {"guide", "Guía", "Flujos de trabajo y ayuda contextual."},
    {"brand", "Marca", "Identidad y versión de ChxChx Studio."},
};
