# Aphyn examples

Examples and demo projects for the [Aphyn](https://github.com/are-you-fruits-studio/aphyn) game engine.
Each example is a standalone Aphyn project under `examples/<name>/` that you can run with `cargo` or package
with `aphyn build`.

**Work in progress:** the examples are being moved here from the engine repository one by one.

## Examples

| Example | What it shows | Run |
|---|---|---|
| [cubes](examples/cubes) | Spinning, sliding cubes: a mesh built in code, unlit material, camera, transform hierarchy | `just dev cubes` |
| [ecs](examples/ecs) | The ECS without rendering: spawning, queries, optional components and resources, despawning (console output) | `just dev ecs` |

`just dev <name>` is `cargo run --manifest-path examples/<name>/scripts/Cargo.toml`.

## License

Code is licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT),
at your option. Assets are listed with their licenses in [ASSETS.md](ASSETS.md).
