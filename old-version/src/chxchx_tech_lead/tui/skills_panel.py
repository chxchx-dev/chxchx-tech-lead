from __future__ import annotations

from pathlib import Path

from textual.widgets import DataTable, Input, Static

from ..skills import SkillRecommendation, SkillRegistry, TechPackRegistry, enabled_skills


class SkillsPanel:
    def _refresh_skills(self) -> None:
        registry = SkillRegistry()
        recommendations = {
            item.skill.name: item
            for item in registry.recommendations(Path(self.project.root))
        }
        active = set(enabled_skills(Path(self.project.root)))
        query = self._query("#skills-search", Input).value
        skills = registry.search(query)
        table = self._query("#skills-table", DataTable)
        table.clear()
        for skill in skills:
            table.add_row(
                "✓" if skill.name in active else "",
                skill.name,
                ", ".join(skill.stacks or skill.languages) or "General",
                skill.description,
                key=skill.name,
            )
        pack_registry = TechPackRegistry()
        matches = pack_registry.detect(Path(self.project.root))
        match_by_name = {match.pack.name: match for match in matches}
        packs = pack_registry.list()
        pack_table = self._query("#packs-table", DataTable)
        pack_table.clear()
        for pack in packs:
            match = match_by_name.get(pack.name)
            pack_table.add_row(
                pack.name,
                ", ".join(match.reasons) if match else "Sin coincidencia automática",
                str(len(pack.skills)),
                key=pack.name,
            )
        self._skill_recommendations = recommendations
        self._skill_active = active
        self._skill_registry = registry
        self._tech_pack_registry = pack_registry
        self._pack_matches = match_by_name
        self._query("#skills-summary", Static).update(
            f"{len(registry.list())} skills en catálogo · {len(active)} habilitadas · "
            f"{len(matches)} de {len(packs)} packs detectados · Proyecto: {self.project.name}"
        )
        self._update_skill_detail()
        self._update_pack_detail()

    def skills_search_changed(self, event: Input.Changed) -> None:
        self._refresh_skills()

    def skill_row_highlighted(self, _event: DataTable.RowHighlighted) -> None:
        self._update_skill_detail()

    def pack_row_highlighted(self, _event: DataTable.RowHighlighted) -> None:
        self._update_pack_detail()

    def _update_skill_detail(self) -> None:
        table = self._query("#skills-table", DataTable)
        name = str(table.coordinate_to_cell_key(table.cursor_coordinate).row_key.value) if table.row_count else ""
        skill = self._skill_registry.get(name) if name else None
        if skill is None:
            self._query("#skill-detail", Static).update("No hay skills que coincidan con la búsqueda.")
            return
        recommendation: SkillRecommendation | None = self._skill_recommendations.get(skill.name)
        reasons = ", ".join(recommendation.reasons) if recommendation else "Sin coincidencia automática"
        self._query("#skill-detail", Static).update(
            f"{skill.name}\n{'Habilitada' if skill.name in self._skill_active else 'Deshabilitada'}"
            f" · {skill.origin}\n\n{skill.description}\n\n"
            f"Stacks: {', '.join(skill.stacks) or '—'}\n"
            f"Lenguajes: {', '.join(skill.languages) or '—'}\n"
            f"Etiquetas: {', '.join(skill.tags) or '—'}\n\n"
            f"Recomendación: {reasons}"
        )

    def _update_pack_detail(self) -> None:
        table = self._query("#packs-table", DataTable)
        name = str(table.coordinate_to_cell_key(table.cursor_coordinate).row_key.value) if table.row_count else ""
        pack = self._tech_pack_registry.get(name) if name else None
        if pack is None:
            self._query("#pack-detail", Static).update("No hay Tech Packs en el catálogo.")
            return
        match = self._pack_matches.get(pack.name)
        self._query("#pack-detail", Static).update(
            f"{pack.name}\n{pack.description}\n\n"
            f"Coincidencias: {', '.join(match.reasons) if match else 'No detectado para este proyecto'}\n\n"
            f"Skills ({len(pack.skills)}):\n" + "\n".join(f"• {name}" for name in pack.skills)
        )
