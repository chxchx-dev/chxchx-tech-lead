from pathlib import Path

from chxchx_tech_lead.adapters.docker.compose import DockerComposeAdapter
from chxchx_tech_lead.core.runner import CommandResult


def test_docker_compose_generates_explicit_project_commands(tmp_path: Path):
    compose = tmp_path / "compose.yaml"
    compose.write_text("services: {}\n", encoding="utf-8")
    calls = []

    def fake_runner(command, **kwargs):
        calls.append((command, kwargs))
        if command[-1] == "version":
            return CommandResult(list(command), 0, "Docker Compose version v2", "")
        return CommandResult(list(command), 0, "ok", "")

    adapter = DockerComposeAdapter(tmp_path, runner=fake_runner, lookup=lambda _: "/usr/bin/docker")

    result = adapter.up()

    assert result.returncode == 0
    assert result.command[:4] == ["docker", "compose", "-f", str(compose)]
    assert result.command[4:] == ["up", "-d"]
    assert calls[-1][1]["cwd"] == tmp_path


def test_docker_stats_parser_ignores_malformed_lines():
    parsed = DockerComposeAdapter.parse_stats(
        "api\t120MiB / 1GiB\t2.3%\nmalformed\nworker\t40MiB / 1GiB\t0.1%\n"
    )

    assert [(item.name, item.cpu_percent) for item in parsed] == [("api", "2.3%"), ("worker", "0.1%")]
