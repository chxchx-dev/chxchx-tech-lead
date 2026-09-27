"""Proceso ligero que mantiene visible el header del workspace en Zellij."""

from __future__ import annotations

import argparse
import sys
import time


def run_header(text: str, interval: float = 30.0) -> None:
    line = f"  {text.replace(chr(10), ' ').strip()}  "
    try:
        while True:
            sys.stdout.write(f"\033[2K\r\033[1;36m{line}\033[0m")
            sys.stdout.flush()
            time.sleep(interval)
    except KeyboardInterrupt:
        sys.stdout.write("\033[2K\r")
        sys.stdout.flush()


def main() -> None:
    parser = argparse.ArgumentParser(description="Header compacto de ChxChx para Zellij")
    parser.add_argument("--text", required=True)
    parser.add_argument("--interval", type=float, default=30.0)
    args = parser.parse_args()
    run_header(args.text, interval=max(args.interval, 1.0))


if __name__ == "__main__":
    main()
