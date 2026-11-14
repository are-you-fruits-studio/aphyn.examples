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
| [boids](examples/boids) | A hundred triangles flocking: a simulation split into ordered systems, settings in a resource | `just dev boids` |
| [gizmos](examples/gizmos) | Debug lines in clip, window and world space, with a fly camera (WASD, R/F, Q/E) | `just dev gizmos` |
| [collisions](examples/collisions) | Collider grid picked with the mouse, and an overlap test between a box and a rotated box | `just dev collisions` |
| [text](examples/text) | Bitmap-font text in world space, rewritten every frame with the elapsed time | `just dev text` |
| [ui](examples/ui) | Rect-based UI: nested and masked panels, a laid-out column, a label, a square following the mouse | `just dev ui` |
| [babble](examples/babble) | Typed text turned into procedural gibberish speech, played back while a circle pulses with its loudness; sliders for speed, pitch, vibrato and brightness | `just dev babble` |

`just dev <name>` is `cargo run --manifest-path examples/<name>/scripts/Cargo.toml`.

## License

Code is licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT),
at your option. Assets are listed with their licenses in [ASSETS.md](ASSETS.md).
