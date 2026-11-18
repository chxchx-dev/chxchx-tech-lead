"""Read chat inputs passed directly or through bounded host-created files."""

from pathlib import Path

from .service import WorkspaceOperationError


def load_prompt(prompt: str, prompt_file: Path | None) -> str:
    """Read a prompt from an option or a file when it would not fit on argv."""
    if prompt_file is None:
        if not prompt.strip():
            raise WorkspaceOperationError("Indica el mensaje con --prompt o --prompt-file")
        return prompt
    try:
        return prompt_file.read_text(encoding="utf-8")
    except (OSError, UnicodeError) as exc:
        raise WorkspaceOperationError(f"No pude leer el archivo del prompt: {exc}") from exc
