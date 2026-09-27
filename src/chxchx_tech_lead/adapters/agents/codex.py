from .base_cli import CliAgentAdapter


class CodexAdapter(CliAgentAdapter):
    def __init__(self, **kwargs):
        super().__init__("codex", "codex", **kwargs)
