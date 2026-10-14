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
- **PNG depth:** 16-bit by default, with an 8-bit option (issue #4, `PngBitDepth`) for engines and importers that mishandle 16-bit PNGs. It applies to Faces and the panorama; JPEG is always 8-bit with a quality setting (default 95). The choice is shown only when PNG is selected.
- **Radiance `.hdr`** was first left out because OpenEXR covers the HDRI requirement. It is now an alternative HDRI format (issue #3, `HdriFormat`): same linear float panorama, written as RGBE by the `image` crate's `hdr` encoder. RGBE has 8-bit mantissas, so it is coarser than 32-bit EXR; EXR stays the default. The export panel shows `.exr` / `.hdr` beside the HDRI checkbox.
- **`image` crate** with only the `png`, `jpeg` and `exr` features, to keep compile time down.
- **Look tuning:** star presence 0.45, brightness skew 9 after viewing renders; `examples/dump_sky.rs` renders a sample sky to PNG for this.

### Memory limits and the app's building blocks (slice 10)

- **Banded rendering.** wgpu's default maximum buffer is 256 MiB, but an 8192 x 8192 float Face reads back 1 GiB. `Renderer::render` therefore renders horizontal bands of about 64 MiB each (`BAND_BUDGET_BYTES`) and stitches them; the shader maps a pass's rows into the band range (`band` uniform). This is the tiling from the original plan, in one dimension. `with_max_band_rows` exists so tests can force many bands and prove the result is identical to one pass.
- **Panorama width is capped at 8192** (`panorama_width`), so an 8K export is 8192 x 4096. Wider would need 2 GiB+ textures; revisit with a tiled panorama if needed.
- **Star radius is capped at one third of a grid cell** (`1 / (3 * STAR_GRID)`). Found by a banding test: when the one-pixel floor made stars wider than a cell on very small renders (Faces under about 300 px), the 3-cell search clipped them and pixels on cell boundaries flipped between renders. Consequence: renders under about 300 px per Face show stars a little smaller than the pixel floor.
- **Preview camera** (`camera_basis`): yaw from +Z toward +X, pitch up, vertical field of view; the Preview reuses the shader through a `FaceBasis`, so no separate preview shader exists.
- **Randomize** lives in the app (`seed_source::fresh_seed`: clock plus counter through a SplitMix64 finalizer) so `spacegen-core` stays deterministic.
- **App dependencies:** `eframe` 0.36 (pulls `wgpu` 30.0.1, the same version as `spacegen-render`) and `rfd` for native open/save/folder dialogs.

### The app (slice 11, the MVP window)

- **One window, three areas:** a left control panel (project, seed, layer list, selected Layer, export), a status line, and the Preview filling the rest. No menu bar: egui menus add little for this few commands.
- **Preview** is the export shader drawn through an egui wgpu paint callback sharing the app's device; dragging moves the sky one pixel per pixel at any zoom; scroll zooms (field of view 0.2 to 2.4 radians).
- **Export runs on a background thread** with its own `Renderer::gpu()` (a second device on the same GPU) and reports progress over a channel, so the window stays responsive. It cannot be cancelled in the MVP.
- **Unsaved changes** are shown by a `*` in the window title, found by comparing the project JSON with the last saved JSON. There is no "save changes?" prompt on close or New in the MVP.
- **Tint** values are used as linear multipliers exactly as the color picker shows them; no gamma conversion between picker and shader.
- **New project** starts with a random seed and one default star field Layer, so the Preview is never empty.
- **Starting view:** yaw 0.4, pitch 0.15, field of view 1.2 radians (about 69 degrees).
- **Run in release mode** (documented in the README); there is no installer or packaging yet.
- **Verified by running it:** the window launches on the GPU, shows the controls and a star field Preview. Clicking through every control and a full export from the window were not automated (egui has no test harness here); the export function itself is covered by tests.

### Nebula Layer (slice 12)

- **Why now:** the original brief (Q2) named nebulae, and a sky of only dots reads as a placeholder; one cloud kind makes the MVP look like space. Galaxy band and sun stay undrawn.
- **Settings:** `coverage`, `scale`, `brightness`, `color_a`, `color_b`, all 0.0 to 1.0, defaults 0.5 and a blue (0.2, 0.4, 1.0) to pink (1.0, 0.3, 0.6) palette. Same validation, save/load, undo and drag-merge rules as star fields.
- **Algorithm:** 6-octave value-noise fbm on the 3D view direction (so it is seamless across Faces and the panorama), thresholded by coverage, with a second slower noise choosing between the two colors. Same integer hash as the stars, so it is deterministic per Seed. All look constants sit at the top of `sky.wgsl`.
- **Layer uniform grew** to 64 bytes (`kind`, `color_a`+opacity, `color_b`) to carry both kinds in one array; still at most 16 visible Layers.
- **Faces versus panorama test:** `render_consistency.rs` looks each panorama pixel up in the matching Face using the Vulkan formulas and compares per Face. A first, weaker "seam" test and a whole-image average both let a mirrored Face through; the per-Face mean (about 0.01 normally, 0.10 or more for a broken Face) is what caught the mutations.
- **Starter project** is now a nebula under a star field.

### Verified by the user

- **Blender import works** (reported by the user): the exported HDRI/panorama loads in Blender. Not yet reported: whether the sky is upright in Blender (our panorama is Y-up with the centre toward +Z; Blender is Z-up), and whether the six Faces tile correctly in Unity, Unreal or Godot. Open items in `docs/research/engine-cubemap-conventions.md` stay open until those are checked.
- **Godot panorama works** (reported by the user, 2026-10-14): the exported panorama in a `PanoramaSkyMaterial` shows no seam and looks right. This confirms the equirectangular orientation decision above against Godot. The six Faces in Godot are still unchecked.
- **Faces work in Unity, Unreal and Godot** (reported by the user, 2026-10-14): the exported Faces tile correctly in all three with no per-engine flip or rotation, so the "names only" Engine convention decision stands and no transform is needed. Closes issue #1. The per-face flip cells for Unity and Unreal in the research file stay unconfirmed from primary sources (issue #5), but the observed result is that none is needed.
- **Panorama is upright in Blender** (reported by the user, 2026-10-14): no orientation option is needed, so the Y-up panorama stays as the only orientation. Closes issue #2.
- **Engine-convention research closed** (2026-10-14, issue #5): the items still marked UNCONFIRMED in `docs/research/engine-cubemap-conventions.md` (Unity panoramic centre, shader property names and handedness; Unreal `.exr` long-lat and equirect centre; Blender handedness) are accepted as open. They do not affect any export seen so far: the Faces work in Unity, Unreal and Godot, and the panorama works in Godot and Blender. The panorama has not been tried in Unity or Unreal; if it looks rotated or mirrored there, open a new issue.

## Continuous integration (issue #21)

- **One Linux job** (`.github/workflows/ci.yml`, `ubuntu-latest`) on every push and pull request: `cargo fmt --all --check`, `cargo build`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`. The project is meant to work on all platforms, so CI is not tied to Windows; Linux is the cheapest runner and the render tests run on Mesa's `lavapipe` software Vulkan driver (installed in the first step). Issue #21 said "Windows"; that was superseded by the all-platforms intent. Windows and macOS can be added as matrix entries later. Format runs first because it is the cheapest to fail. Alternatives: separate jobs per step (each would rebuild; one job shares the cache).
- **Stable Rust, not a pinned toolchain.** New stable releases can add clippy lints and break CI without a code change. That is accepted as cheap to fix (it happened once already: `chunks_exact` and `collapsible_if` were fixed when CI was added) and avoids a pin to maintain. Revisit if it becomes noisy.
- **Skipped render tests fail the run.** Render tests print `SKIPPED` and return when no software adapter exists, which would pass CI while testing nothing. The Test step runs with `--nocapture` and fails if `SKIPPED` appears.
- **Cache:** `Swatinem/rust-cache`. Actions are pinned to major tags, not commit hashes, to keep upkeep low.
- **Windows and macOS run only on a published release** (user decision, 2026-10-14, to save Actions minutes). The `platforms` job builds and tests on `windows-latest` and `macos-latest` when a release is published; the Linux `check` job (with fmt and clippy) covers every push and pull request and is skipped on releases. fmt and clippy are not repeated on the other platforms. This job is untested until a release: Windows should have a software adapter (WARP), but macOS may not, and the SKIPPED guard will then fail that leg. If so, decide between skipping render tests on macOS and another adapter. `fail-fast` is off so one platform's failure does not hide the other's.

## Workflow

- **Pull requests, not `main`** (user decision, 2026-10-14). Work goes on a branch and reaches `main` through a PR, so CI runs before merge. This replaces the earlier "commit locally, never push" rule in `CLAUDE.md`. PRs are not merged without the user's say-so.

## Galaxy band Layer (issue #6)

- **Settings:** `tilt`, `width`, `brightness`, `dust` (dust-lane strength), `color_a` (bright core) and `color_b` (outer glow), all 0.0 to 1.0 with the defaults tilt 0.3, width 0.4, brightness 0.5, dust 0.5, a warm core and cool edges. Validation, save/load, duplicate and undo drag merging copy the star field and nebula Layers.
- **Shape:** one great circle across the sky. The plane's tilt swings from the horizon (0.0) toward vertical (1.0); its direction around the sky is fixed, so the user places the band with tilt and the Seed changes only its structure (clumps, grain, dust breaks). Alternative: a Seed-driven rotation, rejected because the same tilt would then land somewhere different per Seed and the placement controls would stop meaning anything.
- **Look:** a Gaussian glow across the band (width sets its sigma), modulated by low-frequency clumps and a fine grain; dust lanes are a narrower Gaussian broken up by noise that subtracts light; color blends from `color_b` at the edge to `color_a` in the core. All of it is a function of the view direction only, so the Faces and the panorama agree (covered by the consistency test).
- **Old files:** `GalaxyBandSettings` has `#[serde(default)]`, so a file saved when the band was a bare, undrawn Layer loads with the defaults. No format version bump.
- **"Lit" in tests:** the glow is a Gaussian that fades toward zero without reaching it, so the width test counts pixels above 0.02 instead of any non-black pixel.

## Randomize also recolors (user request, 2026-10-14)

- **The Randomize button** now sets a fresh Seed and gives every Layer new colors, as one undo step (`History::randomize`, `Project::randomize_colors`). The colors are derived deterministically from the new Seed, so the core crate stays deterministic and a Seed always gives the same colors.
- **Which colors:** star field tint, both nebula colors and both galaxy band colors. Nothing else changes (opacity, blend, visibility and the other settings stay).
- **How:** random hue with fixed ranges rather than random RGB, which gives muddy greys. Nebula and galaxy band colors are vivid (saturation 0.55 to 1.0, value 0.8 to 1.0) and the second color's hue sits 0.15 to 0.5 around the wheel from the first. Star tints stay near white (saturation up to 0.3), because tinting every star strongly looks wrong. Alternative: fully random star tints, rejected for that reason; easy to change in `ColorPicker::pastel`.
- Colors the user picked by hand are overwritten on Randomize; undo restores them.
