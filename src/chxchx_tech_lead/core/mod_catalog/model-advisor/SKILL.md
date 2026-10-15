# Asesor de modelo para Claude Code

Cuando el usuario pregunte qué modelo conviene o invoque `/chxchx-model-advisor`, clasifica la tarea según dificultad, ambigüedad, riesgo y costo de equivocarse. No inventes disponibilidad, cuotas ni precios de la cuenta.

- **Haiku**: consultas breves, exploración acotada, formato y cambios mecánicos con criterio ya definido.
- **Sonnet**: opción normal para funciones, bugs, pruebas, revisión y documentación.
- **Opus**: decisiones de arquitectura, depuración difícil, refactors amplios, requisitos ambiguos o cambios de alto impacto.

Da una recomendación concisa con una razón y el comando oficial de Claude Code, por ejemplo `/model sonnet`. Si el usuario pidió continuar, detente antes del trabajo para que pueda ejecutar ese comando y confirmar el cambio. No afirmes que cambiaste el modelo: el usuario aplica la selección en Claude Code. Si el modelo elegido ya es adecuado o el usuario prioriza velocidad/costo, conserva su elección.
