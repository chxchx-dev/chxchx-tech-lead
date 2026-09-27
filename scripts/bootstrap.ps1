$ErrorActionPreference = "Stop"

$Repo = "git+https://github.com/chxchx-dev/chxchx-tech-lead.git"
if (-not (Get-Command uv -ErrorAction SilentlyContinue)) {
    irm https://astral.sh/uv/install.ps1 | iex
}

$env:Path = "$env:USERPROFILE\.local\bin;$env:Path"
uv tool install --force $Repo
chxchx-tech install
chxchx-tech doctor
