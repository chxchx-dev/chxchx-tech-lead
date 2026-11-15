# Python engineering

- Keep modules focused and below 300 lines; extract domain behavior into named modules.
- Use type hints for public functions and dataclasses for small immutable domain records.
- Keep CLI and UI callbacks thin; put use cases in domain services and external effects behind adapters.
- Preserve existing public interfaces during incremental refactors.
