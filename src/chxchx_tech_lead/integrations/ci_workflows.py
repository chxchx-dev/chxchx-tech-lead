"""Generate opt-in, least-privilege GitHub Actions CI workflows."""

from __future__ import annotations

import json
import re
import tomllib
from dataclasses import dataclass
from pathlib import Path

from ..core.detector import detect_project

WORKFLOW_RELATIVE_PATH = Path(".github/workflows/chxchx-ci.yml")
CHECKOUT_ACTION = "actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1"
SETUP_UV_ACTION = "astral-sh/setup-uv@1c37ad07a6a961277cf70c0d37d6f313000f5884 # v10.3.0"
SETUP_NODE_ACTION = "actions/setup-node@949feb2413d6458794dcd2491c4babbbce0c15c1 # v7.1.0"
INSTALL_QT_ACTION = "jurplel/install-qt-action@48d3ad6db93f3627c8ee7a0454bc6f3744f7e730 # v4.3.1"


@dataclass(frozen=True, slots=True)
class CiWorkflowPlan:
    path: Path
    stacks: tuple[str, ...]
    content: str
    warnings: tuple[str, ...]


def plan_ci_workflow(root: Path) -> CiWorkflowPlan:
    project = root.resolve()
    info = detect_project(project)
    jobs: list[str] = []
    supported: list[str] = []
    warnings: list[str] = []

    if (project / "pyproject.toml").is_file() or any(
        (project / name).is_file() for name in ("requirements.txt", "requirements-dev.txt")
    ):
        jobs.append(_python_job(project))
        supported.append("Python")
    if (project / "CMakeLists.txt").is_file():
        is_qt = "qt" in info.stacks
        jobs.append(_cmake_job(project, is_qt))
        supported.append("CMake + Qt" if is_qt else "CMake/C++")
        if is_qt:
            warnings.append("Qt usa 6.12.0 como base editable; ajústala si necesitas otra versión.")
    elif any(project.glob("*.pro")):
        warnings.append("Qt qmake detectado; la plantilla automática aún requiere CMake.")
    if (project / "package.json").is_file():
        jobs.append(_node_job(project))
        supported.append("Node.js")
        package = _read_package(project / "package.json")
        if not any(package.get("scripts", {}).get(name) for name in ("test", "lint", "typecheck", "build")):
            warnings.append("package.json no declara scripts test/lint/typecheck/build; CI instalará y verificará dependencias.")
        if not any((project / name).is_file() for name in ("package-lock.json", "npm-shrinkwrap.json")):
            warnings.append("Falta lockfile de npm; se usará npm install y conviene versionar package-lock.json.")

    if not jobs:
        detected = ", ".join(info.stacks) or "ninguno"
        raise ValueError(
            f"No encontré un build compatible para CI automático (stacks: {detected}). "
            "Esta plantilla cubre Python, Node.js y CMake/C++/Qt."
        )

    body = _workflow_document(jobs)
    for warning in warnings:
        body = f"# ChxChx: {warning}\n" + body
    return CiWorkflowPlan(WORKFLOW_RELATIVE_PATH, tuple(supported), body, tuple(warnings))


def _workflow_document(jobs: list[str]) -> str:
    header = """name: ChxChx CI

on:
  push:
  pull_request:
  workflow_dispatch:

permissions:
  contents: read

concurrency:
  group: chxchx-ci-${{ github.workflow }}-${{ github.ref }}
  cancel-in-progress: true

"""
    return header + "jobs:\n" + "\n".join(jobs).rstrip() + "\n"


def _python_job(root: Path) -> str:
    has_pyproject = (root / "pyproject.toml").is_file()
    lock = (root / "uv.lock").is_file()
    requirements = [name for name in ("requirements.txt", "requirements-dev.txt") if (root / name).is_file()]
    commands: list[str] = []
    if has_pyproject:
        sync_flag = " --locked" if lock else ""
        commands.append(f"uv sync{sync_flag} --all-groups")
        run_prefix = "uv run --locked" if lock else "uv run"
        if (root / "tests").is_dir():
            commands.append(f"{run_prefix} --with pytest pytest")
        compile_targets = [name for name in ("src", "tests", "scripts") if (root / name).is_dir()]
        if compile_targets:
            commands.append(f"{run_prefix} python -m compileall {' '.join(compile_targets)}")
    else:
        req_args = " ".join(f"--with-requirements {name}" for name in requirements)
        if (root / "tests").is_dir():
            commands.append(f"uv run --no-project {req_args} --with pytest pytest")
        compile_targets = [name for name in ("src", "tests", "scripts") if (root / name).is_dir()]
        if compile_targets:
            commands.append(f"uv run --no-project {req_args} python -m compileall {' '.join(compile_targets)}")
        if not commands:
            requirement_args = " ".join(f"--requirements {name}" for name in requirements)
            commands.append(f"uv pip install --system {requirement_args}")
    if not commands:
        commands.append("uv --version")

    versions = _python_versions(root)
    version_matrix = ", ".join(f"'{version}'" for version in versions)
    lines = [
        "  python:",
        "    name: Python · ${{ matrix.python-version }}",
        "    runs-on: ubuntu-24.04",
        "    strategy:",
        "      fail-fast: false",
        "      matrix:",
        f"        python-version: [{version_matrix}]",
        "    steps:",
        f"      - uses: {CHECKOUT_ACTION}",
        f"      - uses: {SETUP_UV_ACTION}",
        "        with:",
        "          python-version: ${{ matrix.python-version }}",
        "          enable-cache: true",
        "      - name: Install and verify",
        "        run: |",
    ]
    lines.extend(f"          {command}" for command in commands)
    return "\n".join(lines)


def _python_versions(root: Path) -> tuple[str, ...]:
    supported_versions = [f"3.{minor}" for minor in range(8, 16)]
    defaults = tuple(version for version in supported_versions if int(version.split(".")[1]) >= 11)
    pyproject = root / "pyproject.toml"
    if not pyproject.is_file():
        return defaults
    try:
        project = tomllib.loads(pyproject.read_text(encoding="utf-8")).get("project", {})
    except (OSError, UnicodeError, tomllib.TOMLDecodeError):
        return defaults
    if not isinstance(project, dict):
        return defaults
    requires = project.get("requires-python", "")
    if not isinstance(requires, str):
        return defaults
    if not requires:
        return defaults
    candidates = [
        version
        for version in supported_versions
        if _supports_python_version(version, requires)
    ]
    if not candidates:
        raise ValueError(
            f"requires-python ({requires}) no coincide con las versiones Python 3.8–3.15 "
            "que cubre la plantilla; revisa la matriz manualmente."
        )
    return tuple(candidates)


def _supports_python_version(version: str, requires: str) -> bool:
    """Evaluate common PEP 440 major/minor bounds without runtime dependencies."""
    minor = int(version.split(".")[1])
    clauses = [clause.strip() for clause in requires.split(",")]
    if not clauses or any(not clause for clause in clauses):
        raise ValueError(f"requires-python inválido ({requires}); corrígelo antes de generar CI.")
    for clause in clauses:
        match = re.fullmatch(r"(>=|<=|==|!=|>|<|~=)\s*(\d+)(?:\.(\d+)(\.\*)?)?", clause)
        if not match:
            raise ValueError(
                f"No puedo convertir requires-python ({requires}) a una matriz segura; "
                "edita la versión de Python manualmente en el workflow."
            )
        operator, major_text, minor_text, wildcard = match.groups()
        major = int(major_text)
        bound = int(minor_text) if minor_text else 0
        if wildcard and operator not in {"==", "!="}:
            raise ValueError(f"No puedo convertir requires-python ({requires}) a una matriz segura.")
        if operator == "~=" and not minor_text:
            raise ValueError(f"No puedo convertir requires-python ({requires}) a una matriz segura.")
        candidate_value = (3, minor)
        bound_value = (major, bound)
        if operator == ">=":
            valid = candidate_value >= bound_value
        elif operator == ">":
            valid = candidate_value > bound_value
        elif operator == "<=":
            valid = candidate_value <= bound_value
        elif operator == "<":
            valid = candidate_value < bound_value
        elif operator == "==":
            valid = candidate_value[0] == bound_value[0] and (
                candidate_value[1] == bound_value[1] if minor_text else True
            )
        elif operator == "!=":
            valid = not (
                candidate_value[0] == bound_value[0]
                and (candidate_value[1] == bound_value[1] if minor_text else True)
            )
        else:  # ~= uses the requested major/minor as a lower bound.
            valid = candidate_value >= bound_value and candidate_value[0] == bound_value[0]
        if not valid:
            return False
    return True


def _cmake_job(root: Path, is_qt: bool) -> str:
    lines = [
        "  native-build:",
        "    name: CMake build · ${{ matrix.os }}",
        "    runs-on: ${{ matrix.os }}",
        "    strategy:",
        "      fail-fast: false",
        "      matrix:",
        "        os: [ubuntu-24.04, windows-2025, macos-15]",
        "    steps:",
        f"      - uses: {CHECKOUT_ACTION}",
    ]
    if is_qt:
        lines.extend((
            "      - name: Install Qt 6",
            f"        uses: {INSTALL_QT_ACTION}",
            "        with:",
            "          version: '6.12.0'",
            "          target: desktop",
        ))
        if "core5compat" in _cmake_text(root):
            lines.append("          modules: qt5compat")
    lines.extend((
        "      - name: Configure",
        "        run: cmake -S . -B build -DBUILD_TESTING=ON -DCMAKE_BUILD_TYPE=Release",
        "      - name: Build",
        "        run: cmake --build build --config Release --parallel 2",
        "      - name: Test",
        "        run: ctest --test-dir build --build-config Release --output-on-failure",
    ))
    return "\n".join(lines)


def _node_job(root: Path) -> str:
    package = _read_package(root / "package.json")
    scripts = package.get("scripts", {})
    commands = [f"npm run {name} --if-present" for name in ("test", "lint", "typecheck", "build") if scripts.get(name)]
    if not commands:
        commands = ["npm --version"]
    has_lock = any((root / name).is_file() for name in ("package-lock.json", "npm-shrinkwrap.json"))
    install = "npm ci" if has_lock else "npm install"
    node_version = ".nvmrc" if (root / ".nvmrc").is_file() else "24"
    lines = [
        "  node:",
        "    name: Node.js",
        "    runs-on: ubuntu-24.04",
        "    steps:",
        f"      - uses: {CHECKOUT_ACTION}",
        f"      - uses: {SETUP_NODE_ACTION}",
        "        with:",
    ]
    if node_version == ".nvmrc":
        lines.append("          node-version-file: .nvmrc")
    else:
        lines.append("          node-version: '24'")
    if has_lock:
        lines.append("          cache: npm")
    lines.extend((
        "      - name: Install dependencies",
        f"        run: {install}",
        "      - name: Run project checks",
        "        run: |",
    ))
    lines.extend(f"          {command}" for command in commands)
    return "\n".join(lines)


def _read_package(path: Path) -> dict:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError):
        return {}
    return value if isinstance(value, dict) else {}


def _cmake_text(root: Path) -> str:
    try:
        return (root / "CMakeLists.txt").read_text(encoding="utf-8", errors="ignore").casefold()
    except OSError:
        return ""
