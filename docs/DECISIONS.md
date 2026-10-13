# Decisions log

Smaller decisions made while building. Hard-to-reverse trade-offs get an ADR in `docs/adr/` instead. Each entry: the choice, the alternatives, and why.

## Up to slice 8b (decided with the user)

These were settled in conversation and are recorded here for completeness.

- **Crates:** `spacegen-core` (data model, no GPU), `spacegen-render` (WGSL + wgpu), `spacegen` (the egui app).
- **Project file:** pretty JSON `.spacegen` with `format_version` (currently 1; the shape changed in place before any release).
- **Validation:** out-of-range values are rejected, never clamped.
- **Undo:** snapshot-based `History`, 100 steps, consecutive opacity or star field settings changes to one Layer merge into one step.
- **Engine conventions:** file names only (Unity, Unreal, Godot); Face pixels are always in the Khronos/Vulkan orientation (see `docs/research/engine-cubemap-conventions.md`).
- **Render tests** run on the software adapter and check invariants, not exact pixels.

## From the MVP build onward (decided autonomously)

The user said: "from here on, you will have agency… just get me a MVP. Document your own decisions."

### Star field shader (slice 8c)

- **One shader, one pass, all Layers.** `sky.wgsl` loops over up to 16 visible star field Layers in a single fullscreen pass and blends in the shader (additive or screen). Alternatives: one draw per Layer with GPU blending (needs the `FLOAT32_BLENDABLE` feature, absent on some adapters). Layers beyond 16 are ignored.
- **Same shader everywhere.** Face renders, the equirectangular render and the Preview use one pipeline and uniform layout (`SkyPipeline`, `SkyUniforms`), so what you see is what you export.
- **Star placement:** hashed 3D grid (`STAR_GRID` = 60 cells per unit direction). A star is only kept if its cell lies within 0.5 of the unit sphere, so every star sits within the 27-cell neighbourhood of any direction near it and never gets cut off at cell edges. Integer PCG hash on `u32` (Q80); the Layer's 64-bit sub-seed is passed as two `u32`.
- **Star size floor:** angular radius is clamped to at least 0.9 of a pixel so low-resolution Previews keep their stars (Q83).
- **Look constants** are at the top of `sky.wgsl` and are the place to tune the sky.
- **Equirectangular orientation:** the image centre looks toward +Z and +X is a quarter turn to the right, copying Godot's sky shader. Y is up. This differs from Blender (Z-up, centre at +X), which the research file flags as unconfirmed for the other engines; revisit when verified in an engine.
- **Renderer constructors:** `Renderer::software()` (tests, reproducible) and `Renderer::gpu()` (app).
- **Image viewing during development:** the PPM writer from Q75 is dropped. The `image` crate is needed for Export anyway, so I add it now and view PNGs directly.

### Export (slice 9)

- **Where it lives:** `spacegen-render::export`, because it needs rendered pixels; it takes a `Renderer`, a `Project`, a folder and `ExportOptions`, and reports progress through a callback so the app can run it on a thread.
- **Outputs:** six Faces named by Engine convention, one equirectangular panorama (`<name>_panorama.png|jpg`, width = 4 x face size, so equator pixel density matches the Faces) and an OpenEXR HDRI (`<name>.exr`, 32-bit float, linear, same panorama).
- **Color:** PNG and JPEG are sRGB-encoded from linear light (the sRGB curve runs in `f64`, because the `f32` version turned pure white into 65534). EXR stays linear and keeps values above 1.0.
- **PNG depth:** 16-bit always; no 8-bit PNG option in the MVP. JPEG is 8-bit with a quality setting (default 95).
- **Radiance `.hdr`** is not offered; OpenEXR covers the HDRI requirement and Blender and the major engines read it.
- **`image` crate** with only the `png`, `jpeg` and `exr` features, to keep compile time down.
- **Look tuning:** star presence 0.45, brightness skew 9 after viewing renders; `examples/dump_sky.rs` renders a sample sky to PNG for this.
