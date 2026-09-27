"""Visible launcher used inside agent panes."""

from __future__ import annotations

import os
import subprocess
import sys


def banner(name: str, label: str = "CHXCHX TECH", logo: str = "") -> str:
    title = name.upper()
    identity = " ".join(part for part in (logo, label) if part).strip()
    caption = f"{identity} · {title}"
    return "\n".join(
        (
            "╭──────────────────────────────╮",
            f"│  {caption:<28} │",
            "╰──────────────────────────────╯",
        )
    )


def main() -> int:
    args = sys.argv[1:]
    try:
        name_index = args.index("--name")
        name = args[name_index + 1]
        label = args[args.index("--label") + 1] if "--label" in args else "CHXCHX TECH"
        logo = args[args.index("--logo") + 1] if "--logo" in args else ""
        separator = args.index("--", name_index + 2)
    except (ValueError, IndexError):
        print("Uso: agent_pane --name ID -- COMANDO [ARGUMENTOS...]", file=sys.stderr)
        return 2
    command = args[separator + 1 :]
    if not command:
        print("El comando del agente está vacío", file=sys.stderr)
        return 2
    print(f"\033[1;36m{banner(name, label, logo)}\033[0m", flush=True)
    print(flush=True)
    completed = subprocess.run(command, cwd=os.getcwd(), check=False)
    return completed.returncode


if __name__ == "__main__":
    raise SystemExit(main())
