"""Public CLI entry point; command implementations live by domain."""

from . import commands as _commands  # noqa: F401
from .cli_registry import app

__all__ = ["app"]


if __name__ == "__main__":
    app()
