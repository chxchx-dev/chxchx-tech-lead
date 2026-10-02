from __future__ import annotations

import os
import tempfile

from ...core.runner import CommandResult


class ZellijLayoutOperations:
    """Apply generated layouts as named tabs without attaching a hidden client."""

    command: str

    def add_layout_tab(self, name: str, layout: str, dry_run: bool = False) -> CommandResult:
        if dry_run:
            return CommandResult(
                [self.command, "--session", name, "action", "new-tab", "--layout", "<layout generado>"],
                0,
                "DRY RUN",
                "",
            )
        path = None
        try:
            with tempfile.NamedTemporaryFile(
                mode="w", suffix=".kdl", prefix="chxchx-layout-", encoding="utf-8", delete=False
            ) as stream:
                stream.write(layout)
                path = stream.name
            return self._runner(
                [self.command, "--session", name, "action", "new-tab", "--layout", path]
            )
        except OSError as exc:
            return CommandResult(
                [self.command, "--session", name, "action", "new-tab"],
                2,
                "",
                f"No pude preparar el layout de Zellij: {exc}",
            )
        finally:
            if path:
                try:
                    os.unlink(path)
                except OSError:
                    pass
