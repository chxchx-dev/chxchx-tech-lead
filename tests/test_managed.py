from pathlib import Path

from chxchx_tech_lead.core.managed import upsert_managed_block


def test_managed_block_is_idempotent(tmp_path: Path):
    target = tmp_path / "AGENTS.md"
    assert upsert_managed_block(target, "x", "hello") is True
    first = target.read_text(encoding="utf-8")
    assert upsert_managed_block(target, "x", "hello") is False
    assert target.read_text(encoding="utf-8") == first


def test_managed_block_preserves_manual_text(tmp_path: Path):
    target = tmp_path / "AGENTS.md"
    target.write_text("manual\n", encoding="utf-8")
    upsert_managed_block(target, "x", "managed")
    content = target.read_text(encoding="utf-8")
    assert "manual" in content
    assert "managed" in content
