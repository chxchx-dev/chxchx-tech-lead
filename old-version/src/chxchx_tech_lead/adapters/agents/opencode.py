from .base_cli import CliAgentAdapter


class OpenCodeAdapter(CliAgentAdapter):
    def __init__(self, **kwargs):
        super().__init__("opencode", "opencode", **kwargs)
