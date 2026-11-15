from __future__ import annotations

from ..adapters.editor.base import EditorAdapter
from ..adapters.editor.studio import StudioAdapter
from ..adapters.editor.sublime import SublimeAdapter
from ..core.models import ProjectInfo
from ..core.runner import CommandResult
from .manager import WorkspaceInspection
from .service_models import WorkspaceOperationError


class WorkspaceEditorOperations:
    """Opens the configured editor and keeps Sublime setup as a legacy option."""

    project: ProjectInfo
    _editor: EditorAdapter | None

    def inspect(self) -> WorkspaceInspection: ...

    def open_editor(self, dry_run: bool = False) -> CommandResult:
        inspection = self.inspect()
        result = self._editor_adapter(inspection).open_project(self.project.root, dry_run=dry_run)
        if result.returncode != 0:
            raise WorkspaceOperationError(result.stderr or "No se pudo abrir el editor configurado")
        return result

    def open_studio(self, dry_run: bool = False) -> CommandResult:
        self.inspect()
        result = StudioAdapter().open_project(self.project.root, dry_run=dry_run)
        if result.returncode != 0:
            raise WorkspaceOperationError(result.stderr or "No se pudo abrir ChxChx Studio")
        return result

    def setup_editor(self, dry_run: bool = False, open_after: bool = True):
        self.inspect()
        adapter = SublimeAdapter()
        project_file, generated = adapter.generate_project(self.project.root, dry_run=dry_run)
        if generated.returncode != 0:
            raise WorkspaceOperationError(generated.stderr or "No pude generar el proyecto Sublime")
        opened = None
        if open_after:
            opened = adapter.open_project(project_file, dry_run=dry_run)
            if opened.returncode != 0:
                raise WorkspaceOperationError(opened.stderr or "No pude abrir el proyecto Sublime")
        return project_file, generated, opened

    def _editor_adapter(self, inspection: WorkspaceInspection) -> EditorAdapter:
        if self._editor is not None:
            return self._editor
        editor_name = inspection.config.editor if inspection.config is not None else "studio"
        if editor_name == "studio":
            self._editor = StudioAdapter()
        elif editor_name == "sublime":
            self._editor = SublimeAdapter()
        else:
            raise WorkspaceOperationError(f"Editor no soportado: {editor_name}")
        return self._editor
