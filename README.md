# SpaceGen

A desktop app that generates procedural space skyboxes for game engines and 3D tools.
Edit a sky live, then export it as six cube faces, an equirectangular panorama and an HDRI.

## Run it

You need [Rust](https://rustup.rs) (the stable toolchain, with the MSVC build tools on Windows).

```bash
cargo run --release -p spacegen
```

Use `--release`: the sky is drawn on the GPU, but unoptimized builds make the window sluggish.

## Using it

- **Look around** in the Preview: drag to turn, scroll to zoom.
- **Seed** decides the sky. Type one or press *Randomize*; the same seed and settings always give the same sky.
- **Layers** stack bottom to top. Each has visibility, opacity and a blend mode (additive or screen).
  *Star field* has density, brightness, star size, color variety and a tint.
- **Undo / Redo** (Ctrl+Z, Ctrl+Y or Ctrl+Shift+Z) cover every edit. **Save** (Ctrl+S) writes a `.spacegen` file.
- **Export** writes into a folder you choose:
  - six cube faces named for Unity, Unreal or Godot, as 16-bit PNG or JPEG, 512 to 8192 pixels;
  - one equirectangular panorama (up to 8192 wide);
  - an OpenEXR HDRI (`.exr`, floating point) for Blender and engines that take HDR skies.

  Face pixels always use the Khronos/Vulkan cubemap orientation; only the file names change per engine.
  See `docs/research/engine-cubemap-conventions.md` for what is and is not confirmed about each engine.

## Develop it

```bash
cargo test            # core logic, rendering invariants (on the software GPU adapter), export
cargo run --release -p spacegen-render --example dump_sky -- <folder> [face size]
```

- `crates/spacegen-core`: the data model (Project, Layers, undo/redo, save/load). No GPU code.
- `crates/spacegen-render`: the WGSL sky shader, offscreen rendering, export.
- `crates/spacegen-app`: the egui window.

Read `CONTEXT.md` for the vocabulary, `docs/adr/` for the big decisions and `docs/DECISIONS.md` for the smaller ones.
Work is test-first; see `CLAUDE.md`.
