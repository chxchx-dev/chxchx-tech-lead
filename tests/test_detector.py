from pathlib import Path

from chxchx_tech_lead.core.detector import detect_project


def test_detect_nextjs(tmp_path: Path):
    (tmp_path / "package.json").write_text('{"dependencies":{"next":"1","react":"1"}}', encoding="utf-8")
    (tmp_path / "app.tsx").write_text("export default {}", encoding="utf-8")
    info = detect_project(tmp_path)
    assert "nextjs" in info.stacks
    assert "typescript" in info.languages
    assert info.profile_name == "nextjs"


def test_detect_dotnet(tmp_path: Path):
    (tmp_path / "Demo.sln").write_text("", encoding="utf-8")
    (tmp_path / "Program.cs").write_text("class Program {}", encoding="utf-8")
    info = detect_project(tmp_path)
    assert "dotnet" in info.stacks
    assert "csharp" in info.languages


def test_docker_files_do_not_activate_infrastructure_detection(tmp_path: Path):
    (tmp_path / "Dockerfile").write_text("FROM python:3.13\n", encoding="utf-8")
    (tmp_path / "compose.yaml").write_text("services: {}\n", encoding="utf-8")

    info = detect_project(tmp_path)

    assert "docker" not in info.infrastructure
    assert "postgres" not in info.stacks

def test_detect_next_dotnet_postgres_profile(tmp_path: Path):
    (tmp_path / "package.json").write_text(
        '{"dependencies":{"next":"1","react":"1","pg":"1"}}',
        encoding="utf-8",
    )
    (tmp_path / "App.sln").write_text("", encoding="utf-8")

    info = detect_project(tmp_path)

    assert "postgres" in info.stacks
    assert info.profile_name == "next-dotnet-postgres"


def test_detect_dotnet_postgres_from_npgsql(tmp_path: Path):
    (tmp_path / "App.sln").write_text("", encoding="utf-8")
    (tmp_path / "App.csproj").write_text(
        '<Project><PackageReference Include="Npgsql" Version="1" /></Project>',
        encoding="utf-8",
    )

    info = detect_project(tmp_path)

    assert "postgres" in info.stacks
    assert info.profile_name == "dotnet"



def test_detect_react_native_suggests_pnpm_start(tmp_path: Path):
    import json

    from chxchx_tech_lead.core.config_migrations import default_config_data

    (tmp_path / "package.json").write_text(
        json.dumps({
            "dependencies": {"react-native": "1.0.0"},
            "scripts": {"start": "react-native start"},
        }),
        encoding="utf-8",
    )
    (tmp_path / "pnpm-lock.yaml").write_text("lockfileVersion: '9.0'\n", encoding="utf-8")

    info = detect_project(tmp_path)
    process = default_config_data(info)["workspace"]["processes"][0]

    assert info.profile_name == "react-native"
    assert process["command"] == ["pnpm", "start"]
    assert process["auto_start"] is True
    assert process["cwd"] == "."


def test_detect_rust_and_go_start_commands(tmp_path: Path):
    from chxchx_tech_lead.core.config_migrations import default_config_data

    (tmp_path / "Cargo.toml").write_text("[package]\nname='demo'\n", encoding="utf-8")
    rust = detect_project(tmp_path)
    assert "rust" in rust.stacks
    assert default_config_data(rust)["workspace"]["processes"][0]["command"] == ["cargo", "run"]

    other = tmp_path / "other"
    other.mkdir()
    (other / "go.mod").write_text("module example.com/demo\n", encoding="utf-8")
    (other / "main.go").write_text("package main\n", encoding="utf-8")
    go = detect_project(other)
    assert "go" in go.stacks
    assert default_config_data(go)["workspace"]["processes"][0]["command"] == ["go", "run", "."]


def test_detect_prisma_redis_and_docker(tmp_path: Path) -> None:
    (tmp_path / "package.json").write_text(
        '{"dependencies":{"@prisma/client":"^6","ioredis":"^5","react":"^19"}}',
        encoding="utf-8",
    )
    (tmp_path / "Dockerfile").write_text("FROM node:22\n", encoding="utf-8")
    (tmp_path / "main.tsx").write_text("export const App = () => null\n", encoding="utf-8")

    info = detect_project(tmp_path)

    assert {"react", "prisma", "redis", "docker"}.issubset(info.stacks)
    assert "typescript" in info.languages

def test_detect_python_redis_dependency(tmp_path: Path) -> None:
    (tmp_path / "pyproject.toml").write_text(
        '[project]\nname = "sample"\ndependencies = ["redis>=5"]\n',
        encoding="utf-8",
    )

    info = detect_project(tmp_path)

    assert {"python", "redis"}.issubset(info.stacks)
