from .base_cli import CliAgentAdapter


class ClaudeAdapter(CliAgentAdapter):
    def __init__(self, **kwargs):
        super().__init__("claude", "claude", **kwargs)
