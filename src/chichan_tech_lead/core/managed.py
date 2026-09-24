from __future__ import annotations

import re
from pathlib import Path

START = "<!-- chichan:start {name} -->"
END = "<!-- chichan:end {name} -->"


def render_block(name: str, body: str) -> str:
    return f"{START.format(name=name)}\n{body.rstrip()}\n{END.format(name=name)}"


def upsert_managed_block(path: Path, name: str, body: str, dry_run: bool = False) -> bool:
    new_block = render_block(name, body)
    old = path.read_text(encoding="utf-8") if path.exists() else ""
    start = START.format(name=name)
    end = END.format(name=name)
    pattern = re.compile(re.escape(start) + r".*?" + re.escape(end), re.DOTALL)

    if pattern.search(old):
        new = pattern.sub(new_block, old, count=1)
    else:
        prefix = old.rstrip()
        new = (prefix + "\n\n" if prefix else "") + new_block + "\n"

    if new == old:
        return False
    if not dry_run:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(new, encoding="utf-8")
    return True
