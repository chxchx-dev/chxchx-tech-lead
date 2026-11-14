"""Read-only Zellij pane for local context estimates and provider quota commands."""

from __future__ import annotations

import argparse
import time
from datetime import datetime
from pathlib import Path

from rich import box
from rich.console import Console, Group
from rich.live import Live
from rich.panel import Panel
from rich.table import Table
from rich.text import Text

from ..integrations.agent_usage import (
    LiveAgentUsage,
    LocalContextUsage,
    latest_context_usage,
    read_live_usage,
)


def _context_label(usage: LocalContextUsage | None, live: LiveAgentUsage | None) -> str:
    if live and (live.context_remaining_percent is not None or live.context_used_percent is not None):
        used = live.context_used_percent
        remaining = (
            live.context_remaining_percent
            if live.context_remaining_percent is not None
            else 100 - (used or 0)
        )
        used_text = f"{used:.0f}% usado" if used is not None else f"{remaining:.0f}% libre"
        return f"{used_text} · {remaining:.0f}% libre"
    if usage is None or (usage.input_tokens is None and usage.used_percent is None):
        return "Sin datos de contexto locales"
    if usage.used_percent is not None:
        remaining = max(0, 100 - round(usage.used_percent))
        return f"{round(usage.used_percent)}% usado · {remaining}% libre"
    if usage.input_tokens is not None:
        return f"{usage.input_tokens:,} tokens del último turno · total no disponible"
    return "Contexto no disponible"


def _reset_label(timestamp: float | None) -> str:
    if timestamp is None:
        return ""
    return " · " + datetime.fromtimestamp(timestamp).astimezone().strftime("%H:%M")


def _quota_label(provider: str, live: LiveAgentUsage | None) -> str:
    if provider == "codex":
        return "Pie Codex · /status"
    if provider == "claude":
        if live is None:
            return "Esperando datos de sesión · /usage"
        windows = []
        if live.five_hour_used_percent is not None:
            remaining = max(0, 100 - round(live.five_hour_used_percent))
            windows.append(f"5h {remaining}% libre{_reset_label(live.five_hour_resets_at)}")
        if live.seven_day_used_percent is not None:
            remaining = max(0, 100 - round(live.seven_day_used_percent))
            windows.append(f"7d {remaining}% libre{_reset_label(live.seven_day_resets_at)}")
        return " · ".join(windows) if windows else "No reportado · /usage"
    return "No disponible desde esta CLI"


def render(root: Path, agents: list[tuple[str, str]]) -> Panel:
    table = Table(box=box.SIMPLE, expand=True, padding=(0, 1))
    table.add_column("AGENTE", style="bold cyan", no_wrap=True)
    table.add_column("CONTEXTO DEL CHAT", overflow="fold")
    table.add_column("CUOTA DEL PLAN", overflow="fold")
    for agent_id, provider in agents:
        live = read_live_usage(root, agent_id)
        has_live_context = live is not None and (
            live.context_remaining_percent is not None or live.context_used_percent is not None
        )
        usage = None if has_live_context else latest_context_usage(root, provider)
        table.add_row(agent_id, _context_label(usage, live), _quota_label(provider, live))

    note = Text(
        "Se actualiza desde métricas locales del agente · sin consultas de red",
        style="dim",
    )
    return Panel(
        Group(table, note),
        title="USO DE AGENTES",
        subtitle="Se actualiza cada 3 s · Codex /status · Claude /usage",
        border_style="bright_blue",
        padding=(0, 1),
    )


def _parse_agent(value: str) -> tuple[str, str]:
    agent_id, separator, provider = value.partition("=")
    if not separator or not agent_id.strip():
        raise argparse.ArgumentTypeError("Formato esperado: ID=PROVEEDOR")
    return agent_id.strip(), provider.strip().casefold()


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", required=True, type=Path)
    parser.add_argument("--agent", action="append", default=[], type=_parse_agent)
    args = parser.parse_args()
    if not args.agent:
        print("No hay agentes configurados para mostrar.")
        return 0

    console = Console()
    with Live(render(args.root, args.agent), console=console, refresh_per_second=1, screen=False) as live:
        try:
            while True:
                time.sleep(3)
                live.update(render(args.root, args.agent), refresh=True)
        except KeyboardInterrupt:
            return 0


if __name__ == "__main__":
    raise SystemExit(main())
