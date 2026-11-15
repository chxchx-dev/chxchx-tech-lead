"""Run a destructive-operation-free end-to-end lab flow in a temporary directory."""

from __future__ import annotations

import os
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "src"))

from typer.testing import CliRunner  # noqa: E402

from chxchx_tech_lead.cli import app  # noqa: E402


def invoke(runner: CliRunner, args: list[str]):
    result = runner.invoke(app, args)
    if result.exit_code != 0:
        raise RuntimeError(f"{args} failed ({result.exit_code}):\n{result.stdout}")
    return result


def main() -> None:
    runner = CliRunner()
    with tempfile.TemporaryDirectory(prefix="chxchx-tech-lab-") as temp:
        root = Path(temp)
        project = root / "next-nest-postgres"
        project.mkdir()
        (project / "package.json").write_text(
            '{"dependencies":{"next":"1","react":"1","@nestjs/core":"1","pg":"1"}}',
            encoding="utf-8",
        )
        (project / "app.tsx").write_text("export default function App() {}\n", encoding="utf-8")
        (project / "compose.yml").write_text(
            "services:\n  db:\n    image: postgres:16\n", encoding="utf-8"
        )
        global_home = root / "global"
        old_home = os.environ.get("CHXCHX_TECH_HOME")
        os.environ["CHXCHX_TECH_HOME"] = str(global_home)
        try:
            setup_preview = invoke(runner, ["setup", str(project), "--dry-run"])
            assert "No se realizaron cambios" in setup_preview.stdout
            assert not (project / ".ai").exists()
            assert not global_home.exists()

            preview = invoke(runner, ["init", str(project), "--dry-run"])
            assert "No se realizaron cambios" in preview.stdout
            assert not (project / ".ai").exists()
            assert not global_home.exists()

            first = invoke(runner, ["init", str(project)])
            second = invoke(runner, ["init", str(project)])
            assert "next-nest-postgres" in first.stdout
            assert "Todo está actualizado" in second.stdout
            assert (project / ".ai" / "chxchx-tech.toml").exists()

            agents = project / "AGENTS.md"
            original = agents.read_text(encoding="utf-8")
            agents.write_text(original + "\nManual lab note.\n", encoding="utf-8")
            no_change_sync = invoke(runner, ["sync", str(project)])
            assert "sin cambios" in no_change_sync.stdout
            assert "Manual lab note." in agents.read_text(encoding="utf-8")

            drifted = agents.read_text(encoding="utf-8").replace(
                "Perfil detectado: **next-nest-postgres**",
                "Perfil detectado: **manual-drift**",
            )
            agents.write_text(drifted, encoding="utf-8")
            sync = invoke(runner, ["sync", str(project)])
            assert "backup" in sync.stdout.lower()
            assert "Manual lab note." in agents.read_text(encoding="utf-8")
            assert "manual-drift" not in agents.read_text(encoding="utf-8")

            invoke(runner, ["projects", "list"])
            invoke(runner, ["projects", "sync", "--dry-run"])
            invoke(runner, ["rollback", str(project)])
            restored = agents.read_text(encoding="utf-8")
            assert "manual-drift" in restored
            assert "Manual lab note." in restored
        finally:
            if old_home is None:
                os.environ.pop("CHXCHX_TECH_HOME", None)
            else:
                os.environ["CHXCHX_TECH_HOME"] = old_home
    print("lab smoke: ok")


if __name__ == "__main__":
    main()
