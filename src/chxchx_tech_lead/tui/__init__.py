"""Interfaz opcional de Terminal Workspace basada en Textual."""

from .app import TUIUnavailableError, run_tui

__all__ = ["TUIUnavailableError", "run_tui"]
