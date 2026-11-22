from __future__ import annotations

import importlib.util
from pathlib import Path
import xml.etree.ElementTree as ET


REPOSITORY = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    "windows_msi_generator", REPOSITORY / "scripts" / "generate_windows_msi_wxs.py"
)
assert SPEC is not None and SPEC.loader is not None
GENERATOR = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(GENERATOR)
WIX = "http://wixtoolset.org/schemas/v4/wxs"


def test_msi_runs_required_extras_and_keeps_the_shared_tools_path(tmp_path: Path) -> None:
    payload = tmp_path / "payload"
    cli = payload / "bin" / "chxchx-tech.exe"
    cli.parent.mkdir(parents=True)
    cli.write_bytes(b"cli")
    (payload / ".chxchx-managed").write_text("a" * 40, encoding="utf-8")
    (payload / ".chxchx-msi").write_text("channel=stable\n", encoding="utf-8")
    output = tmp_path / "ChxChxStudio.wxs"

    GENERATOR.generate(
        payload,
        output,
        channel="stable",
        version="1.2.3",
        commit="a" * 40,
        project_root=tmp_path,
    )

    root = ET.parse(output).getroot()
    action = root.find(f"{{{WIX}}}Package/{{{WIX}}}CustomAction[@Id='InstallRequiredTools']")
    assert action is not None
    assert action.attrib == {
        "Id": "InstallRequiredTools",
        "FileRef": GENERATOR._id("file", "stable/stable-1.2.3/bin/chxchx-tech.exe"),
        "ExeCommand": 'tools install --bundle "[INSTALLFOLDER]required-tools"',
        "Execute": "deferred",
        "Impersonate": "yes",
        "Return": "check",
    }
    sequence = root.find(f"{{{WIX}}}Package/{{{WIX}}}InstallExecuteSequence/{{{WIX}}}Custom")
    assert sequence is not None
    assert sequence.attrib == {
        "Action": "InstallRequiredTools",
        "After": "InstallFiles",
        "Condition": 'NOT REMOVE="ALL"',
    }
    tools_component = root.find(f".//{{{WIX}}}Component[@Id='SharedToolsPath']")
    assert tools_component is not None and tools_component.attrib["Permanent"] == "yes"
    assert tools_component.find(f"{{{WIX}}}CreateFolder") is not None
    assert tools_component.find(f"{{{WIX}}}RemoveFolder[@On='uninstall']") is not None
    tools_path = tools_component.find(f"{{{WIX}}}Environment[@Id='SharedToolsPathEntry']")
    assert tools_path is not None and tools_path.attrib["Permanent"] == "yes"
    assert tools_path.attrib["Value"] == "[SharedToolsFolder]"
