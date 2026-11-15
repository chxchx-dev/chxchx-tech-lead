"""Curated local coding models for OpenCode, with sizes from the Ollama library.

OpenCode needs tool calling, so only models the library lists with the
`tools` capability appear here. The list is best-first: the recommendation
takes the first model that fits. Sizes are the download sizes shown on
ollama.com/library and are approximate (checked 2026-11-02).
"""

from __future__ import annotations

from dataclasses import dataclass

GIB = 1024**3


@dataclass(frozen=True, slots=True)
class CatalogModel:
    tag: str
    label: str
    size_gb: float
    context_k: int
    note: str

    @property
    def size_bytes(self) -> int:
        return int(self.size_gb * GIB)


CATALOG: tuple[CatalogModel, ...] = (
    CatalogModel("gpt-oss:120b", "gpt-oss 120B", 65.0, 128, "Muy grande: requiere workstation o más de 64 GB de memoria."),
    CatalogModel("qwen3-coder:30b", "Qwen3 Coder 30B", 19.0, 256, "Modelo de código de uso general, el mejor equilibrio si cabe."),
    CatalogModel("devstral:24b", "Devstral 24B", 14.0, 128, "Pensado para agentes de código."),
    CatalogModel("gpt-oss:20b", "gpt-oss 20B", 14.0, 128, "Razonamiento y herramientas."),
    CatalogModel("qwen2.5-coder:32b", "Qwen2.5 Coder 32B", 20.0, 32, "Código, ventana de 32K."),
    CatalogModel("qwen2.5-coder:14b", "Qwen2.5 Coder 14B", 9.0, 32, "Opción media para 16 GB de memoria."),
    CatalogModel("qwen2.5-coder:7b", "Qwen2.5 Coder 7B", 4.7, 32, "Ligero; razona menos y falla más con herramientas."),
    CatalogModel("qwen2.5-coder:3b", "Qwen2.5 Coder 3B", 1.9, 32, "Mínimo viable; solo tareas simples."),
)


def find_model(tag: str) -> CatalogModel | None:
    return next((model for model in CATALOG if model.tag == tag), None)
