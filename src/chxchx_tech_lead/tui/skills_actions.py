from __future__ import annotations

from pathlib import Path

from textual.widgets import DataTable

from ..skills import enable_skills, set_skill_enabled, sync_project_skills


class SkillsActions:
    def button_skills_refresh(self) -> None:
        self._refresh_skills()

    def button_packs_refresh(self) -> None:
        self._refresh_skills()

    def button_skill_enable(self) -> None:
        self._set_selected_skill(True)

    def button_skill_disable(self) -> None:
        self._set_selected_skill(False)

    def button_skills_sync(self) -> None:
        project = Path(self.project.root)
        try:
            preview = sync_project_skills(project, dry_run=True)
            if not preview.changed:
                self._set_log("El contexto de skills ya está sincronizado.")
                self.notify("No hay cambios que sincronizar", severity="information")
                return
            result = sync_project_skills(project)
        except (OSError, ValueError) as exc:
            self._set_log(f"No se pudo sincronizar skills: {exc}")
            self.notify(str(exc), severity="error")
            return
        self._set_log(f"Contexto sincronizado · {len(result.enabled)} skills")
        self.notify("Contexto de skills sincronizado", severity="information")

    def button_packs_sync(self) -> None:
        self.button_skills_sync()

    def button_pack_apply(self) -> None:
        table = self._query("#packs-table", DataTable)
        if not table.row_count:
            self.notify("No hay Tech Packs detectados", severity="warning")
            return
        key = str(table.coordinate_to_cell_key(table.cursor_coordinate).row_key.value)
        pack = self._tech_pack_registry.get(key)
        if pack is None:
            self.notify("Selecciona un Tech Pack", severity="warning")
            return
        self._apply_pack_skills(pack.name, pack.skills)

    def button_packs_apply_detected(self) -> None:
        names = tuple(
            skill for match in self._pack_matches.values() for skill in match.pack.skills
        )
        if not names:
            self.notify("No hay packs compatibles con este proyecto", severity="warning")
            return
        self._apply_pack_skills("packs detectados", names)

    def _set_selected_skill(self, enabled: bool) -> None:
        table = self._query("#skills-table", DataTable)
        if not table.row_count:
            self.notify("No hay una skill seleccionada", severity="warning")
            return
        name = str(table.coordinate_to_cell_key(table.cursor_coordinate).row_key.value)
        project = Path(self.project.root)
        try:
            preview = set_skill_enabled(project, name, enabled, dry_run=True)
            if not preview.changed:
                message = "La skill ya está habilitada" if enabled else "La skill ya estaba deshabilitada"
                self.notify(message, severity="information")
                return
            set_skill_enabled(project, name, enabled)
        except (OSError, ValueError) as exc:
            self._set_log(f"No se pudo cambiar {name}: {exc}")
            self.notify(str(exc), severity="error")
            return
        state = "habilitada" if enabled else "deshabilitada"
        self._set_log(f"Skill {state}: {name}. Sincroniza para actualizar el contexto.")
        self.notify(f"Skill {state}: {name}", severity="information")
        self._refresh_skills()

    def _apply_pack_skills(self, label: str, names: tuple[str, ...]) -> None:
        project = Path(self.project.root)
        try:
            preview = enable_skills(project, names, dry_run=True)
            if not preview.changed:
                self.notify(f"{label.capitalize()} ya estaban aplicados", severity="information")
                return
            result = enable_skills(project, names)
        except (OSError, ValueError) as exc:
            self._set_log(f"No se pudo aplicar {label}: {exc}")
            self.notify(str(exc), severity="error")
            return
        added = sorted(set(result.enabled) - self._skill_active)
        self._set_log(f"{label.capitalize()}: {', '.join(added)}. Sincroniza para cargar el contexto.")
        self.notify(f"{label.capitalize()} aplicados · {len(added)} skills nuevas", severity="information")
        self._refresh_skills()
