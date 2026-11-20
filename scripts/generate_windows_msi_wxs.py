#!/usr/bin/env python3
"""Generate per-user WiX components for a fully staged Windows payload."""

from __future__ import annotations

import argparse
from pathlib import Path
import re
import uuid
import xml.etree.ElementTree as ET

WIX = "http://wixtoolset.org/schemas/v4/wxs"
ET.register_namespace("", WIX)
NAMESPACE = uuid.UUID("69f84af7-7b64-4e3e-9f2f-0d687f40e0f9")
UPGRADE_CODES = {
    "stable": "{7697B2CE-EB03-47C0-BED0-11A328DD7761}",
    "dev": "{B53D454C-D004-40DB-94D1-A1AC07A246C0}",
}
COMMIT_PATTERN = re.compile(r"[0-9a-f]{40}")
VERSION_PATTERN = re.compile(r"[0-9]+\.[0-9]+\.[0-9]+")


def _tag(namespace: str, name: str) -> str:
    return f"{{{namespace}}}{name}"


def _id(prefix: str, value: str) -> str:
    return f"{prefix}_{uuid.uuid5(NAMESPACE, value).hex[:20]}"


def _component_guid(channel: str, install_name: str, path: str) -> str:
    return "{" + str(uuid.uuid5(NAMESPACE, f"{channel}/{install_name}/{path}")).upper() + "}"


def install_folder(channel: str, version: str, commit: str) -> str:
    if channel == "stable":
        return f"stable-{version}"
    return f"dev-{commit[:12]}"


def generate(
    payload: Path,
    output: Path,
    *,
    channel: str,
    version: str,
    commit: str,
    project_root: Path,
) -> None:
    if channel not in UPGRADE_CODES:
        raise ValueError(f"Canal MSI inválido: {channel}")
    if not VERSION_PATTERN.fullmatch(version):
        raise ValueError(f"La versión MSI debe ser X.Y.Z: {version}")
    if not COMMIT_PATTERN.fullmatch(commit):
        raise ValueError("El commit MSI debe ser un SHA completo de 40 caracteres.")
    payload = payload.resolve(strict=True)
    project_root = project_root.resolve(strict=True)
    files = sorted(path for path in payload.rglob("*") if path.is_file())
    if any(path.is_symlink() for path in payload.rglob("*")):
        raise ValueError("El payload MSI no puede contener enlaces simbólicos.")
    for marker in (".chxchx-managed", ".chxchx-msi"):
        if not (payload / marker).is_file():
            raise ValueError(f"Falta el marcador del payload: {marker}")
    if not files:
        raise ValueError("El payload MSI está vacío.")
    if len(files) > 60_000:
        raise ValueError("El payload MSI excede el límite seguro de componentes.")

    root_name = install_folder(channel, version, commit)
    label = "ChxChx Studio Dev" if channel == "dev" else "ChxChx Studio"
    wix = ET.Element(_tag(WIX, "Wix"))
    package = ET.SubElement(
        wix,
        _tag(WIX, "Package"),
        {
            "Name": label,
            "Manufacturer": "ChxChx",
            "Version": version,
            "UpgradeCode": UPGRADE_CODES[channel],
            "Scope": "perUser",
            "InstallerVersion": "500",
            "Compressed": "yes",
            "Language": "1033",
        },
    )
    ET.SubElement(
        package,
        _tag(WIX, "MajorUpgrade"),
        {
            "AllowSameVersionUpgrades": "yes" if channel == "dev" else "no",
            "Schedule": "afterInstallInitialize",
            "DowngradeErrorMessage": "Ya hay una versión más reciente de ChxChx instalada.",
        },
    )
    ET.SubElement(package, _tag(WIX, "MediaTemplate"), {"EmbedCab": "yes"})
    ET.SubElement(
        package,
        _tag(WIX, "Icon"),
        {"Id": "ChxChxIcon", "SourceFile": "$(var.ProjectRoot)\\assets\\brand\\chxchx.ico"},
    )
    ET.SubElement(package, _tag(WIX, "Property"), {"Id": "ARPPRODUCTICON", "Value": "ChxChxIcon"})
    root_path = f"[LocalAppDataFolder]Programs\\ChxChxStudio\\{root_name}"
    existing_root = ET.SubElement(package, _tag(WIX, "Property"), {"Id": "CHXCHX_EXISTING_INSTALL_ROOT"})
    ET.SubElement(existing_root, _tag(WIX, "DirectorySearch"), {"Id": "ExistingInstallRootSearch", "Path": root_path})
    registered = ET.SubElement(package, _tag(WIX, "Property"), {"Id": "CHXCHX_REGISTERED_MSI"})
    ET.SubElement(
        registered,
        _tag(WIX, "RegistrySearch"),
        {
            "Id": "RegisteredMsiSearch",
            "Root": "HKCU",
            "Key": f"Software\\ChxChx\\Studio\\Installers\\{channel}\\{root_name}",
            "Name": "Installed",
            "Type": "raw",
        },
    )
    ET.SubElement(
        package,
        _tag(WIX, "Launch"),
        {
            "Condition": "REMOVE=\"ALL\" OR NOT CHXCHX_EXISTING_INSTALL_ROOT OR CHXCHX_REGISTERED_MSI",
            "Message": "La carpeta de esta versión ya existe y no pertenece a una instalación MSI registrada. Desinstala la instalación anterior antes de continuar.",
        },
    )
    feature = ET.SubElement(package, _tag(WIX, "Feature"), {"Id": "MainFeature", "Title": label, "Level": "1"})
    ET.SubElement(feature, _tag(WIX, "ComponentGroupRef"), {"Id": "ApplicationFiles"})
    ET.SubElement(feature, _tag(WIX, "ComponentRef"), {"Id": "UserPath"})
    ET.SubElement(feature, _tag(WIX, "ComponentRef"), {"Id": "SharedToolsPath"})
    ET.SubElement(feature, _tag(WIX, "ComponentRef"), {"Id": "StartMenuShortcut"})

    tools_command = _id("file", f"{channel}/{root_name}/bin/chxchx-tech.exe")
    ET.SubElement(
        package,
        _tag(WIX, "CustomAction"),
        {
            "Id": "InstallRequiredTools",
            "FileRef": tools_command,
            "ExeCommand": "tools install",
            "Execute": "deferred",
            "Impersonate": "yes",
            "Return": "check",
        },
    )
    sequence = ET.SubElement(package, _tag(WIX, "InstallExecuteSequence"))
    ET.SubElement(
        sequence,
        _tag(WIX, "Custom"),
        {"Action": "InstallRequiredTools", "After": "InstallFiles", "Condition": 'NOT REMOVE="ALL"'},
    )

    install_fragment = ET.SubElement(wix, _tag(WIX, "Fragment"))
    local = ET.SubElement(install_fragment, _tag(WIX, "StandardDirectory"), {"Id": "LocalAppDataFolder"})
    programs = ET.SubElement(local, _tag(WIX, "Directory"), {"Id": "ProgramsFolder", "Name": "Programs"})
    base = ET.SubElement(programs, _tag(WIX, "Directory"), {"Id": "ChxChxStudioFolder", "Name": "ChxChxStudio"})
    root_directory = ET.SubElement(base, _tag(WIX, "Directory"), {"Id": "INSTALLFOLDER", "Name": root_name})
    ET.SubElement(base, _tag(WIX, "Directory"), {"Id": "SharedToolsFolder", "Name": "tools"})

    directory_elements = {"": root_directory}
    directory_ids = {"": "INSTALLFOLDER"}
    all_directories = sorted(
        {path.relative_to(payload).parent.as_posix() for path in files if path.parent != payload}
    )
    for relative in all_directories:
        parts = relative.split("/")
        for index in range(len(parts)):
            key = "/".join(parts[: index + 1])
            if key in directory_elements:
                continue
            parent_key = "/".join(parts[:index])
            directory_id = _id("dir", f"{channel}/{root_name}/{key}")
            directory = ET.SubElement(
                directory_elements[parent_key],
                _tag(WIX, "Directory"),
                {"Id": directory_id, "Name": parts[index]},
            )
            directory_elements[key] = directory
            directory_ids[key] = directory_id

    files_fragment = ET.SubElement(wix, _tag(WIX, "Fragment"))
    group = ET.SubElement(files_fragment, _tag(WIX, "ComponentGroup"), {"Id": "ApplicationFiles"})
    files_by_directory: dict[str, list[Path]] = {}
    for path in files:
        directory_key = path.parent.relative_to(payload).as_posix()
        files_by_directory.setdefault("" if directory_key == "." else directory_key, []).append(path)

    for directory_key, directory_id in sorted(directory_ids.items(), key=lambda item: (item[0].count("/"), item[0])):
        component_id = _id("cmp", f"{channel}/{root_name}/directory/{directory_key or '.'}")
        component = ET.SubElement(
            group,
            _tag(WIX, "Component"),
            {
                "Id": component_id,
                "Directory": directory_id,
                "Guid": _component_guid(channel, root_name, f"directory/{directory_key or '.'}"),
            },
        )
        component_registry_key = f"Software\\ChxChx\\Studio\\Components\\{channel}\\{root_name}\\{component_id}"
        ET.SubElement(
            component,
            _tag(WIX, "RegistryValue"),
            {
                "Root": "HKCU",
                "Key": component_registry_key,
            "Name": "UserPath",
                "Type": "integer",
                "Value": "1",
                "KeyPath": "yes",
            },
        )
        for path in files_by_directory.get(directory_key, []):
            relative = path.relative_to(payload).as_posix()
            file_id = _id("file", f"{channel}/{root_name}/{relative}")
            source = "$(var.PayloadDir)\\" + relative.replace("/", "\\")
            ET.SubElement(component, _tag(WIX, "File"), {"Id": file_id, "Source": source})
        ET.SubElement(
            component,
            _tag(WIX, "RemoveFolder"),
            {"Id": _id("remove", f"{channel}/{root_name}/directory/{directory_key or '.'}"), "On": "uninstall"},
        )

    for directory_id in ("ProgramsFolder", "ChxChxStudioFolder"):
        component_id = _id("cmp", f"{channel}/{root_name}/parent/{directory_id}")
        component = ET.SubElement(
            group,
            _tag(WIX, "Component"),
            {
                "Id": component_id,
                "Directory": directory_id,
                "Guid": _component_guid(channel, root_name, f"parent/{directory_id}"),
            },
        )
        ET.SubElement(
            component,
            _tag(WIX, "RegistryValue"),
            {
                "Root": "HKCU",
                "Key": f"Software\\ChxChx\\Studio\\Components\\{channel}\\{root_name}\\{component_id}",
                "Name": "Installed",
                "Type": "integer",
                "Value": "1",
                "KeyPath": "yes",
            },
        )
        ET.SubElement(
            component,
            _tag(WIX, "RemoveFolder"),
            {"Id": _id("remove", f"{channel}/{root_name}/parent/{directory_id}"), "On": "uninstall"},
        )

    shared_fragment = ET.SubElement(wix, _tag(WIX, "Fragment"))
    menu = ET.SubElement(shared_fragment, _tag(WIX, "StandardDirectory"), {"Id": "ProgramMenuFolder"})
    menu_folder = ET.SubElement(menu, _tag(WIX, "Directory"), {"Id": "CompanyMenuFolder", "Name": "ChxChx"})
    shortcut_component = ET.SubElement(
        menu_folder,
        _tag(WIX, "Component"),
        {
            "Id": "StartMenuShortcut",
            "Guid": _component_guid(channel, root_name, "start-menu"),
        },
    )
    ET.SubElement(
        shortcut_component,
        _tag(WIX, "Shortcut"),
        {
            "Id": "StudioShortcut",
            "Name": label,
            "Target": "[INSTALLFOLDER]shims\\chxchx-studio.cmd",
            "WorkingDirectory": "INSTALLFOLDER",
            "Icon": "ChxChxIcon",
            "IconIndex": "0",
        },
    )
    ET.SubElement(shortcut_component, _tag(WIX, "RemoveFolder"), {"Id": "RemoveCompanyMenuFolder", "On": "uninstall"})
    ET.SubElement(
        shortcut_component,
        _tag(WIX, "RegistryValue"),
        {
            "Root": "HKCU",
            "Key": f"Software\\ChxChx\\Studio\\Installers\\{channel}\\{root_name}",
            "Name": "StartMenuShortcut",
            "Type": "integer",
            "Value": "1",
            "KeyPath": "yes",
        },
    )
    path_component = ET.SubElement(
        shared_fragment,
        _tag(WIX, "Component"),
        {
            "Id": "UserPath",
            "Directory": "INSTALLFOLDER",
            "Guid": _component_guid(channel, root_name, "user-path"),
        },
    )
    ET.SubElement(
        path_component,
        _tag(WIX, "Environment"),
        {
            "Id": "UserPathEntry",
            "Name": "PATH",
            "Value": "[INSTALLFOLDER]shims",
            "Action": "set",
            "Part": "first",
            "System": "no",
            "Permanent": "no",
        },
    )
    ET.SubElement(
        path_component,
        _tag(WIX, "RegistryValue"),
        {
            "Root": "HKCU",
            "Key": f"Software\\ChxChx\\Studio\\Installers\\{channel}\\{root_name}",
            "Name": "UserPath",
            "Type": "string",
            "Value": "[INSTALLFOLDER]shims",
            "KeyPath": "yes",
        },
    )

    tools_component = ET.SubElement(
        shared_fragment,
        _tag(WIX, "Component"),
        {
            "Id": "SharedToolsPath",
            "Directory": "SharedToolsFolder",
            "Guid": "{" + str(uuid.uuid5(NAMESPACE, "shared/tools-path")).upper() + "}",
            "Permanent": "yes",
        },
    )
    ET.SubElement(tools_component, _tag(WIX, "CreateFolder"))
    ET.SubElement(
        tools_component,
        _tag(WIX, "RemoveFolder"),
        {"Id": "RemoveSharedToolsFolder", "On": "uninstall"},
    )
    ET.SubElement(
        tools_component,
        _tag(WIX, "Environment"),
        {
            "Id": "SharedToolsPathEntry",
            "Name": "PATH",
            "Value": "[SharedToolsFolder]",
            "Action": "set",
            "Part": "first",
            "System": "no",
            "Permanent": "yes",
        },
    )
    ET.SubElement(
        tools_component,
        _tag(WIX, "RegistryValue"),
        {
            "Root": "HKCU",
            "Key": "Software\\ChxChx\\Studio\\Shared",
            "Name": "ToolsPath",
            "Type": "string",
            "Value": "[SharedToolsFolder]",
            "KeyPath": "yes",
        },
    )

    ET.indent(wix, space="  ")
    output.parent.mkdir(parents=True, exist_ok=True)
    ET.ElementTree(wix).write(output, encoding="utf-8", xml_declaration=True)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--payload", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--project-root", type=Path, required=True)
    parser.add_argument("--channel", choices=tuple(UPGRADE_CODES), required=True)
    parser.add_argument("--version", required=True)
    parser.add_argument("--commit", required=True)
    args = parser.parse_args()
    try:
        generate(
            args.payload,
            args.output,
            channel=args.channel,
            version=args.version,
            commit=args.commit,
            project_root=args.project_root,
        )
    except (OSError, ValueError) as error:
        parser.error(str(error))
    print(f"WiX source generated: {args.output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
