# TypeScript engineering

- Keep `strict` type checking enabled and model domain boundaries with explicit types.
- Validate untrusted data at system boundaries; a TypeScript annotation does not validate runtime input.
- Prefer small modules and named domain types over broad `any`, type assertions, or catch-all helpers.
- Keep I/O behind adapters and handle asynchronous failures at the layer that can recover from them.
