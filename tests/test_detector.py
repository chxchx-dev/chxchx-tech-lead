from pathlib import Path

from chichan_tech_lead.core.detector import detect_project


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
