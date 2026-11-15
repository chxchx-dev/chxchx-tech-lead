from pathlib import Path


def test_production_python_modules_stay_below_300_lines() -> None:
    source_root = Path(__file__).parents[1] / "src"
    oversized = [
        f"{path.relative_to(source_root)} ({len(path.read_text(encoding='utf-8').splitlines())} lines)"
        for path in source_root.rglob("*.py")
        if len(path.read_text(encoding="utf-8").splitlines()) >= 300
    ]

    assert not oversized, "Python modules must stay below 300 lines:\n" + "\n".join(oversized)
