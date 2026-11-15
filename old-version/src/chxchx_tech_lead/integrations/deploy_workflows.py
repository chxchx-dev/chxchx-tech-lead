"""Generate opt-in GitHub Actions deployment workflows for Docker Compose VPSs."""

from __future__ import annotations

import re
from dataclasses import dataclass
from pathlib import Path

from .ci_workflows import CHECKOUT_ACTION

WORKFLOW_RELATIVE_PATH = Path(".github/workflows/chxchx-deploy.yml")
COMPOSE_FILES = ("compose.yaml", "compose.yml", "docker-compose.yaml", "docker-compose.yml")


@dataclass(frozen=True, slots=True)
class DeployWorkflowPlan:
    path: Path
    compose_file: str
    auto_deploy_branch: str | None
    content: str
    required_variables: tuple[str, ...]
    required_secrets: tuple[str, ...]


def plan_deploy_workflow(root: Path, auto_deploy_branch: str | None = None) -> DeployWorkflowPlan:
    project = root.resolve()
    compose_files = [name for name in COMPOSE_FILES if (project / name).is_file()]
    if not compose_files:
        raise ValueError(
            "CD a VPS admite por ahora proyectos con Docker Compose. "
            "Añade compose.yaml y vuelve a previsualizar el setup."
        )
    if len(compose_files) > 1:
        raise ValueError(
            "Encontré varios manifiestos Docker Compose. Combínalos en uno solo "
            "para evitar que el workflow despliegue un conjunto incompleto."
        )
    if auto_deploy_branch is not None and not _valid_branch(auto_deploy_branch):
        raise ValueError("La rama de despliegue debe ser un nombre Git válido y no contener comodines.")

    compose_file = compose_files[0]
    body = _workflow_document(auto_deploy_branch)
    return DeployWorkflowPlan(
        WORKFLOW_RELATIVE_PATH,
        compose_file,
        auto_deploy_branch,
        body,
        ("VPS_SSH_HOST", "VPS_SSH_USER", "VPS_DEPLOY_PATH"),
        ("VPS_SSH_PRIVATE_KEY", "VPS_SSH_KNOWN_HOSTS"),
    )


def _workflow_document(auto_deploy_branch: str | None) -> str:
    trigger = "  workflow_dispatch:\n    inputs:\n      confirm_deploy:\n        description: Confirma el despliegue en el VPS\n        type: boolean\n        required: true\n        default: false\n"
    if auto_deploy_branch is not None:
        trigger = f"  push:\n    branches:\n      - '{auto_deploy_branch}'\n" + trigger
    return f"""name: ChxChx Deploy · VPS

on:
{trigger}
permissions:
  contents: read

concurrency:
  group: chxchx-deploy-production
  cancel-in-progress: false

jobs:
  deploy:
    if: github.event_name != 'workflow_dispatch' || inputs.confirm_deploy
    runs-on: ubuntu-24.04
    timeout-minutes: 20
    environment: production
    steps:
      - name: Checkout deployment revision
        uses: {CHECKOUT_ACTION}
        with:
          ref: ${{{{ github.sha }}}}
          persist-credentials: false
      - name: Deploy with Docker Compose over SSH
        env:
          VPS_SSH_HOST: ${{{{ vars.VPS_SSH_HOST }}}}
          VPS_SSH_USER: ${{{{ vars.VPS_SSH_USER }}}}
          VPS_SSH_PORT: ${{{{ vars.VPS_SSH_PORT }}}}
          VPS_DEPLOY_PATH: ${{{{ vars.VPS_DEPLOY_PATH }}}}
          VPS_SSH_PRIVATE_KEY: ${{{{ secrets.VPS_SSH_PRIVATE_KEY }}}}
          VPS_SSH_KNOWN_HOSTS: ${{{{ secrets.VPS_SSH_KNOWN_HOSTS }}}}
          DEPLOY_SHA: ${{{{ github.sha }}}}
          DEPLOY_REF: ${{{{ github.ref }}}}
          DEPLOY_REPOSITORY: ${{{{ github.repository }}}}
        run: |
          set -euo pipefail
          : "${{VPS_SSH_HOST:?Configura la variable VPS_SSH_HOST}}"
          : "${{VPS_SSH_USER:?Configura la variable VPS_SSH_USER}}"
          VPS_SSH_PORT="${{VPS_SSH_PORT:-22}}"
          : "${{VPS_DEPLOY_PATH:?Configura la variable VPS_DEPLOY_PATH}}"
          : "${{VPS_SSH_PRIVATE_KEY:?Configura el secreto VPS_SSH_PRIVATE_KEY}}"
          : "${{VPS_SSH_KNOWN_HOSTS:?Configura el secreto VPS_SSH_KNOWN_HOSTS}}"
          [[ "$VPS_SSH_HOST" =~ ^[A-Za-z0-9.-]+$ && "$VPS_SSH_HOST" != -* ]]
          [[ "$VPS_SSH_USER" =~ ^[A-Za-z0-9._-]+$ && "$VPS_SSH_USER" != -* && "$VPS_SSH_USER" != root ]]
          [[ "$VPS_SSH_PORT" =~ ^[0-9]{{1,5}}$ ]] && (( VPS_SSH_PORT > 0 && VPS_SSH_PORT < 65536 ))
          [[ "$VPS_DEPLOY_PATH" == /* && "$VPS_DEPLOY_PATH" != *$'\\n'* && "$VPS_DEPLOY_PATH" != *$'\\r'* ]]
          key_file="$RUNNER_TEMP/chxchx-vps-key"
          known_hosts_file="$RUNNER_TEMP/chxchx-known-hosts"
          trap 'rm -f "$key_file" "$known_hosts_file"' EXIT
          umask 077
          printf '%s\\n' "$VPS_SSH_PRIVATE_KEY" > "$key_file"
          printf '%s\\n' "$VPS_SSH_KNOWN_HOSTS" > "$known_hosts_file"
          path_b64="$(printf '%s' "$VPS_DEPLOY_PATH" | base64 -w0)"
          ref_b64="$(printf '%s' "$DEPLOY_REF" | base64 -w0)"
          repository_b64="$(printf '%s' "$DEPLOY_REPOSITORY" | base64 -w0)"
          ssh -i "$key_file" -p "$VPS_SSH_PORT" \\
            -o BatchMode=yes -o IdentitiesOnly=yes -o StrictHostKeyChecking=yes \\
            -o UserKnownHostsFile="$known_hosts_file" -o ConnectTimeout=15 \\
            "$VPS_SSH_USER@$VPS_SSH_HOST" \\
            "bash -s -- $path_b64 $ref_b64 $DEPLOY_SHA $repository_b64" <<'REMOTE'
          set -euo pipefail
          decode() {{ printf '%s' "$1" | base64 --decode; }}
          deploy_path="$(decode "$1")"
          deploy_ref="$(decode "$2")"
          target_sha="$3"
          expected_repository="$(decode "$4")"
          [[ "$deploy_path" == /* && -d "$deploy_path/.git" ]]
          [[ "$target_sha" =~ ^[0-9a-f]{{40,64}}$ ]]
          case "$(git -C "$deploy_path" remote get-url origin)" in
            "https://github.com/$expected_repository"|"https://github.com/$expected_repository.git"|\\
            "git@github.com:$expected_repository"|"git@github.com:$expected_repository.git"|\\
            "ssh://git@github.com/$expected_repository"|"ssh://git@github.com/$expected_repository.git") ;;
            *) echo "El remote origin del VPS no coincide con el repositorio desplegado." >&2; exit 1 ;;
          esac
          if ! git -C "$deploy_path" diff --quiet HEAD -- || ! git -C "$deploy_path" diff --cached --quiet; then
            echo "El working tree del VPS tiene cambios versionados; se cancela para preservarlos." >&2
            exit 1
          fi
          git -C "$deploy_path" fetch --prune origin "$deploy_ref"
          git -C "$deploy_path" cat-file -e "$target_sha^{{commit}}"
          previous_sha="$(git -C "$deploy_path" rev-parse HEAD)"
          rollback() {{
            result=$?
            if (( result == 0 )); then return; fi
            trap - EXIT
            echo "El despliegue falló; intento restaurar $previous_sha." >&2
            if git -C "$deploy_path" checkout --detach "$previous_sha" && \\
               (cd "$deploy_path" && docker compose up -d --build); then
              echo "Se restauró la revisión anterior." >&2
            else
              echo "Falló el rollback. Revisa el estado del VPS antes de repetir el despliegue." >&2
            fi
            exit "$result"
          }}
          trap rollback EXIT
          git -C "$deploy_path" checkout --detach "$target_sha"
          cd "$deploy_path"
          docker compose up -d --build
          wait_for_services() {{
            for ((attempt = 0; attempt < 30; attempt++)); do
              states="$(docker compose ps --all --format '{{{{.State}}}}')"
              health="$(docker compose ps --all --format '{{{{.Health}}}}')"
              if [[ -n "$states" ]] && ! grep -qvx 'running' <<<"$states" &&
                 ! grep -qx 'unhealthy' <<<"$health" && ! grep -qx 'starting' <<<"$health"; then
                return 0
              fi
              sleep 2
            done
            echo "Los servicios del VPS no quedaron healthy/running en 60 segundos." >&2
            docker compose ps --all >&2
            return 1
          }}
          wait_for_services
          trap - EXIT
          echo "Despliegue completado: $target_sha"
          REMOTE
"""


def _valid_branch(value: str) -> bool:
    return (
        bool(re.fullmatch(r"[A-Za-z0-9._/-]+", value))
        and not value.startswith("-")
        and ".." not in value
        and "//" not in value
        and not value.endswith(("/", ".", ".lock"))
    )
