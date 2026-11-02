from __future__ import annotations

import json
from pathlib import Path

from chxchx_tech_lead.integrations import agent_catalog


def test_codex_options_come_from_the_local_model_cache_and_config(tmp_path: Path, monkeypatch) -> None:
    monkeypatch.setenv("CODEX_HOME", str(tmp_path))
    (tmp_path / "config.toml").write_text('model = "gpt-b"\nmodel_reasoning_effort = "xhigh"\n', encoding="utf-8")
    (tmp_path / "models_cache.json").write_text(json.dumps({"models": [
        {"slug": "gpt-a", "display_name": "GPT A", "visibility": "list", "default_reasoning_level": "low",
         "supported_reasoning_levels": [{"effort": "low"}, {"effort": "ultra"}]},
        {"slug": "hidden", "visibility": "hide", "supported_reasoning_levels": []},
    ]}), encoding="utf-8")

    options = agent_catalog.agent_options("Codex", "definitely-missing-codex")

    assert options["available"] is False and "npm" in options["install_hint"]
    assert (options["default_model"], options["default_effort"]) == ("gpt-b", "xhigh")
    assert options["models"] == [{"id": "gpt-a", "label": "GPT A", "efforts": ["low", "ultra"], "default_effort": "low"}]


def test_claude_options_use_documented_aliases_and_user_settings(tmp_path: Path, monkeypatch) -> None:
    monkeypatch.setenv("CLAUDE_CONFIG_DIR", str(tmp_path))
    (tmp_path / "settings.json").write_text('{"model": "opus", "effortLevel": "high"}', encoding="utf-8")

    options = agent_catalog.agent_options("Claude", "claude")

    assert options["default_model"] == "opus" and options["default_effort"] == "high"
    assert {"sonnet", "opus", "haiku"} <= {model["id"] for model in options["models"]}


def test_codex_limits_read_the_latest_token_count_of_the_thread(tmp_path: Path, monkeypatch) -> None:
    monkeypatch.setenv("CODEX_HOME", str(tmp_path))
    rollout = tmp_path / "sessions" / "2026" / "11" / "02" / "rollout-2026-11-02T05-00-00-thread-1.jsonl"
    rollout.parent.mkdir(parents=True)
    token_count = {"type": "event_msg", "payload": {"type": "token_count",
        "info": {"last_token_usage": {"input_tokens": 20665}, "model_context_window": 475000},
        "rate_limits": {"primary": {"used_percent": 12.5, "window_minutes": 300, "resets_at": 1793633900},
                        "secondary": {"used_percent": 4.0, "window_minutes": 10080, "resets_at": 1793792783},
                        "plan_type": "plus"}}}
    rollout.write_text("\n".join([json.dumps({"type": "session_meta", "payload": {}}), json.dumps(token_count)]),
                       encoding="utf-8")

    limits = agent_catalog.codex_limits("thread-1")

    assert limits == {
        "context_tokens": 20665, "context_window": 475000,
        "primary": {"used_percent": 12.5, "window_minutes": 300, "resets_at": 1793633900},
        "secondary": {"used_percent": 4.0, "window_minutes": 10080, "resets_at": 1793792783},
        "plan": "plus",
    }
    assert agent_catalog.codex_limits("otro-hilo") is None
