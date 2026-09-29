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
