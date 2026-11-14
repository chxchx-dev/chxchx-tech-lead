from __future__ import annotations

import argparse
from pathlib import Path


SMALL_FILE_COUNT = 20
LARGE_FILE_COUNT = 5_000
FILE_BYTES = 512


def write_project(root: Path, file_count: int) -> None:
    root.mkdir(parents=True, exist_ok=True)
    (root / "README.md").write_text("Synthetic ChxChx performance fixture.\n", encoding="utf-8")
    content = "Synthetic source file for repeatable filesystem indexing.\n"
    content = (content * ((FILE_BYTES // len(content)) + 1))[:FILE_BYTES]
    for index in range(file_count):
        directory = root / "src" / f"group-{index // 100:03d}"
        directory.mkdir(parents=True, exist_ok=True)
        (directory / f"file-{index:05d}.txt").write_text(content, encoding="ascii")

    for excluded in (".git", "target", "node_modules"):
        ignored = root / excluded
        ignored.mkdir(exist_ok=True)
        (ignored / "excluded.txt").write_text("This directory is excluded by Studio.\n", encoding="ascii")


def main() -> None:
    parser = argparse.ArgumentParser(description="Create deterministic, secret-free performance fixtures.")
    parser.add_argument("output", type=Path, help="Output directory; it must not already exist.")
    args = parser.parse_args()
    output = args.output.expanduser().resolve()
    if output.exists():
        parser.error(f"output path already exists: {output}")
    output.mkdir(parents=True)
    write_project(output / "small", SMALL_FILE_COUNT)
    write_project(output / "large", LARGE_FILE_COUNT)
    print(f"small: {SMALL_FILE_COUNT + 1} indexed files under {output / 'small'}")
    print(f"large: {LARGE_FILE_COUNT + 1} indexed files under {output / 'large'}")


if __name__ == "__main__":
    main()
