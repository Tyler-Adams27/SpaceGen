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

## Sun Layer (issue #7)

- **Settings:** `azimuth`, `elevation`, `size`, `brightness`, `glare` and `color`, all 0.0 to 1.0, defaults azimuth 0.0, elevation 0.6, size 0.5, brightness 0.5, glare 0.5, color a warm white. Validation, save/load, duplicate and undo drag merging copy the other Layer kinds.
- **Direction is two angles**, edited as "Direction" (azimuth) and "Height" (elevation) sliders. Azimuth 0.0 looks toward +Z (the panorama's centre), 0.25 toward +X, 0.5 behind; elevation 0.5 is the horizon, 1.0 straight up. Alternative: "aim at the current view", rejected because it ties the model to the Preview camera, cannot be reproduced from a saved Project and cannot be tested through the core API.
- **A Sun is placed by hand, so the Seed does not affect it** (a test pins this). Randomize recolors it (a near-white tint, like star tints) but never moves it.
- **Look:** a disc (radius 0.7 to 9 degrees, never under a pixel) at 8 times white at full brightness, so the HDRI keeps values above 1.0, plus a halo that fades over about six disc radii and a faint wide glare. The glare strength scales both. Tuning constants are at the top of `sky.wgsl`; the wide glare was lowered after a first render lifted the whole sky to grey.
- **Old files:** `SunSettings` has `#[serde(default)]`, like the galaxy band, so a bare Sun Layer from an older file loads with the defaults.
- **Faces-versus-panorama test:** uses six suns (one toward each Face) at low brightness. At full brightness the nearest-pixel comparison of an 8x-white disc edge reported a 0.034 mean difference on one Face; the same scene at lower brightness agrees, and a deliberate pixel-size-dependent mutation still fails by a wide margin (0.14 against the 0.02 limit).

## Per-Layer re-roll (issue #8)

- **A per-Layer `variation` number** (`u64`, saved with the Project, default 0) is mixed into the Layer's sub-seed by `Layer::seed_under`. Variation 0 returns exactly the old `Seed::layer_seed`, so every existing Project and every existing render is unchanged; a re-roll sets a non-zero variation (a Seed of 0 is stored as 1, so a re-roll can never return the original look). Files saved before this field existed load with 0 (`#[serde(default)]`), no format version bump. Alternative: re-rolling by rewriting the Layer's id, rejected because ids are how undo, duplicate and the UI refer to Layers.
- **A re-roll also recolors that Layer**, matching what the whole-Project Randomize does, using the same color rules, and changes nothing else (opacity, blend, visibility, other settings, other Layers, the Project Seed). It is one undo step; two re-rolls are two steps and are never merged like a slider drag.
- **The Sun's structure does not change** on a re-roll (a hand-placed Sun does not use its seed), only its color.
- **The random number comes from the app** (`fresh_seed`), as for Randomize, so the core stays deterministic: `Project::reroll_layer(layer, seed)`.
- **The whole-Project Randomize leaves variations alone.** It changes the Project Seed and every color; a re-rolled Layer still looks different from before because its base seed changed.
- **UI:** a "Re-roll this layer" button under the selected Layer's heading.
- **Tests:** isolation is proved in `render_reroll.rs` by hiding the re-rolled Layer and checking the others' pixels are identical, and a structure test compares a re-roll against the old structure in the new colors, so a color-only change would fail it.

## App icon

- **The window icon is `assets/icon.png`** (256x256 RGBA), embedded with `include_bytes!` and decoded by `eframe::icon_data::from_png_bytes` (`icon::app_icon`), then set with `ViewportBuilder::with_icon`. `assets/logo.svg` is the source: a rounded tile with a neon nebula, galaxy band, a sun with rays and a grey cube standing for the Skybox and its Faces.
- **The PNG is committed, not rendered at build time.** The logo uses SVG filters (noise, displacement, masks) that need a full renderer such as resvg; pulling one into the app's dependencies or a build script would add compile time and a failure point for a picture that rarely changes. Alternative: rasterise in `build.rs`, rejected for that reason. To regenerate: `npm i @resvg/resvg-js`, render `logo.svg` at 256 px width to `icon.png`. The `ray` filter uses an explicit `userSpaceOnUse` region because resvg panicked on a percentage region combined with `feDisplacementMap`.
- **Not done: the Windows `.exe` file icon** (Explorer, installers). That needs an `.ico` embedded by a build script (for example `winres`). The window and taskbar show the icon at runtime; the file icon is a separate change.
- **Tests:** the picture is 256x256 RGBA, with transparent rounded corners and an opaque middle (a solid-square mutation fails it).

## Drag-to-reorder Layers (issue #10)

- **A "::" handle on each Layer row** starts a drag carrying the `LayerId` (egui's drag-and-drop payload). The row under the pointer on release is the new index, passed to the existing undoable `History::move_layer`, so a drag is one undo step. A line marks the drop spot (above the row when moving up the stack, below it when moving down). Up and Down stay as the keyboard fallback.
- **Dragging only from the handle**, not the whole row, so clicking a row to select it and the other row buttons are unaffected. Alternative: whole-row dragging with `dnd_drag_source`, rejected because it competes with selection clicks.
- **Not unit-tested:** this is GUI glue over `move_layer`, which is already covered in `layer_editing.rs` and `history.rs`. Verified only that the app builds, passes clippy and launches; the drag gesture itself was not exercised (no egui test harness, see issue #20).

## Cancel a running export (issue #11)

- **A `CancelToken`** (a shared atomic flag, cloneable) is set on the `Renderer` with `with_cancel_token`. The renderer checks it before each band of rows, and `export_skybox` checks it before each image and before the HDRI step. Alternative: a new parameter on `export_skybox` or a second `export_skybox_cancellable`, rejected because every caller and test would change (or the API would double) for something that is a property of one export run, and the Renderer is already built per export.
- **Result:** a cancelled export returns `ExportError::Cancelled` (a render stopped by the token maps to it too) and **removes the files it had already written**, so a half-finished set of Faces never sits beside an older complete one. A file already being encoded and saved is allowed to finish (then removed), so no half-written file can be left. Alternative: keep the finished files, rejected as a confusing partial result.
- **UI:** a "Cancel" button under the progress bar; the status reads "Cancelling..." then "Export cancelled". Cancelling takes effect after the current GPU band (at most about 64 MiB of rows), not instantly.
- **Tests** (`render/tests/export.rs`): cancelling from the progress callback after two images stops the export and leaves the folder empty; a pre-cancelled token with 1-row bands makes `render_face` fail with `is_cancelled()`. Both were mutation-checked (cleanup disabled, band check disabled). The Cancel button itself is GUI glue and was not clicked in the running app.

## Warn before losing unsaved changes (issue #12)

- **`unsaved::Guard`** holds the action waiting on the user (New, Open or Close) and decides what happens: a clean Project runs it at once; otherwise a "Save / Discard / Cancel" window is shown. Save runs the action only if the save succeeded; a cancelled save dialog or a failed write drops the action and keeps the Project open (the user can retry), instead of re-asking. Alternative: re-show the prompt after a failed save, rejected as a loop with no new information.
- **Window close:** when a close is requested and there are unsaved changes, the close is cancelled (`ViewportCommand::CancelClose`) and the same prompt is shown; Discard or a successful Save then sends the close again with a flag that lets it through.
- **"Unsaved" is the existing `*` rule** (project JSON differs from the last saved JSON), so an edit undone back to the saved state is not unsaved.
- **Not covered:** an export running when the window closes is not asked about (it is stopped with the process and may leave partial files; cancel it first).
- **Tests:** `Guard` is unit-tested through `request`, `resolve` and `pending`, with three mutations checked (Discard dropping the action, Save ignoring the result, the prompt not holding the action). The dialog and the close hook are GUI glue: only the launch was checked, not the clicks.

## Remember recent files and export settings (issue #13)

- **One JSON file**, `settings.json`, in `SpaceGen/` under the platform config directory (`%APPDATA%` on Windows, `~/Library/Application Support` on macOS, `$XDG_CONFIG_HOME` or `~/.config` elsewhere), found from environment variables rather than the `dirs` crate to avoid a dependency for three lines. It holds the recent files (newest first, no duplicates, at most 8) and the whole `ExportSettings` (which moved to `settings.rs` and gained serde derives; the four export enums in the core crate gained them too). `#[serde(default)]` means a file with missing fields keeps the defaults for those.
- **A missing or corrupt file gives the defaults**, never an error. A failed write is ignored silently (settings are a convenience).
- **When it is written:** each frame the app compares the serialized settings with what it last wrote and writes only on a difference, so changes land within a frame and nothing depends on a clean shutdown. Alternative: eframe's `save` hook, rejected because it needs eframe's `persistence` feature (not enabled), and an `on_exit` write, rejected because it is lost on a crash or kill.
- **Recent menu:** a "Recent" menu in the project bar, disabled when empty. Choosing one goes through the same unsaved-changes prompt as Open. A recent file that fails to open is dropped from the list.
- **Not remembered:** window size and position, camera, and the Project itself (the app still starts on the starter Project). The issue title said "window state"; the Done-when list did not, so it was left out.
- **Tests:** `settings.rs` covers the round trip, missing, corrupt and partial files, and the recent list rules; the dedupe and cap were mutation-checked. The Recent menu and the per-frame write are GUI glue and were only checked by launching.

## Preview: flat panorama toggle (issue #14)

- **Two buttons above the Preview**, "Free look" and "Flat panorama" (`PreviewMode`). Flat uses the existing `Projection::Equirectangular`, so it matches the exported panorama pixel for pixel (same shader path). The camera (yaw, pitch, zoom) is kept while in flat mode and is back as it was when switching to Free look.
- **The flat view is the largest centred 2:1 rectangle** that fits the Preview area (`panorama_rect`), because the shader maps the whole rectangle to 360 by 180 degrees and any other shape would stretch the sky. The bars around it show the panel background. It is not draggable or zoomable: the point is to see everything. Alternative: fill the area and stretch, rejected as misleading.
- **The optional Face outline overlay is not done.** Cube edges are curves in an equirectangular image; drawing them correctly is its own piece of work and the issue marks it optional. Left open on #14's "later a Face inspector" line.
- **Tests:** `panorama_rect` (wide, tall and exactly 2:1 areas), written red first. The toggle and the paint call are GUI glue; only a launch was checked, the view was not looked at on screen.

## Stream large exports to disk (issue #15)

- **`Renderer::render_face_rows` / `render_equirectangular_rows`** hand each band of whole rows to a sink (`sink(first_row, pixels)`) as soon as it is read back, so only one band (about 64 MiB of float pixels) is in memory. The sink's error type only has to convert from `RenderError`; a sink error stops the render and comes back unchanged. The old whole-image `render_face` and `render_equirectangular` are built on the same loop.
- **PNG rows are encoded as they arrive** with the `png` crate directly (already in the build through `image`), converting linear light to sRGB per band, 8 or 16 bit. The settings match what the `image` crate used before (fast compression, adaptive filter): the `png` crate's own default was about four times slower on an 8192 export for a small gain in size. Output is pixel-identical to before.
- **JPEG keeps 8-bit RGB in memory** (3 bytes per pixel, a fifth of the float image) and encodes at the end, because the `image` crate's JPEG encoder needs the whole picture. Alternative: a streaming JPEG encoder crate, rejected as a new dependency for a format that is rarely used at 8K. Not measured.
- **HDRI still holds the whole float image** (EXR and Radiance writers take whole images). With "panorama + HDRI" the panorama is saved from that same image; with the panorama alone it streams. The HDRI's memory is why the panorama width cap matters for it (issue #16).
- **Cancel:** a streamed file is written in place, so on any error or cancel `stream_image` removes the unfinished file; the earlier cancel test covers it and was mutation-checked (it fails if the removal is dropped).
- **Tests:** `render_rows.rs` (bands in order, sizes, add up to the whole Face, sink errors); `export.rs` writes PNG 16-bit, PNG 8-bit and JPEG in 3-row bands and checks every file decodes to exactly the in-memory encoding of the same Renderer's render (a mutation that drops a byte per band fails it). It does not compare banded with single-pass renders: on CI's software Vulkan driver those differ by a few star pixels, which ADR 0001 allows.
- **Measured** (`dump_sky <dir> 8192`, release build, Windows, real GPU, six 8192 Faces plus an 8192 panorama as 16-bit PNG; peak working set sampled every 100 ms): **before 2,134 MiB, 9 s; after 271 MiB, 8 s**, 439 MiB of files in both. The remaining memory is mostly the GPU driver and the band buffers. HDRI and JPEG exports at 8192 were not measured.

## Panoramas wider than 8192 (issue #16)

- **Column tiles.** The shader's per-pass slice (`band`) now carries a column range as well as a row range (`SkyUniforms::with_columns`). `Renderer::render_rows` draws each band of rows in tiles no wider than `max_tile_width` (default: the GPU's `max_texture_dimension_2d`, 8192 with wgpu's default limits, which is why the old cap was 8192) and stitches them into the band before handing it to the sink. A tile is a slice of the same view-direction mapping, so tile edges fall mid-sky with no seam.
- **The panorama has no cap**: `panorama_width` is four Faces wide, so an 8192 Face gives 32768 x 16384 (the largest the app offers; the PNG format allows far more). It is always streamed (issue #15), so memory stays at one band. Measured on the real GPU with `dump_sky 8192` (six Faces and that panorama): **peak 324 MiB, 17 s**, panorama file 484 MiB, header checked as 32768 x 16384. JPEG panoramas this wide keep 8-bit RGB in memory (1.5 GiB for 32768 x 16384) and were not tried.
- **The HDRI keeps a cap of 8192** (`hdri_width`): EXR and Radiance writers take the whole float image (512 MiB at 8192 x 4096; 8 GiB at 32768 x 16384). Because the two widths now differ for Faces over 2048, the panorama and the HDRI are rendered separately instead of sharing one render, which costs a second render when both are ticked. Alternative: tile and stream the EXR (scanline blocks) too, left for later.
- **Tests.** `render_rows.rs`: a 64-wide panorama in 11-wide tiles (the last one narrower), with 5-row bands, matches a single pass by mean difference below 0.002 (exact equality fails on some drivers, see ADR 0001); shifting a tile edge by one pixel gives 0.016, which fails it. `export.rs`: `panorama_width` / `hdri_width` values, and the streamed-file test now runs with 24-wide tiles so the panorama file is written from tiled bands.

## Clear failures, no overwrite surprises (issue #17)

- **Plain-language messages.** `ExportError::user_message()` and `RenderError::user_message()` return a sentence saying what happened and what to try; `Display` stays technical for logs. Covered: permission denied, folder missing or not a folder, disk full, read-only folder, system out of memory, no usable GPU, GPU stopped responding, GPU out of memory, cancelled; anything else says "Could not write the export files: <system text>" or "Rendering failed: <detail>" so support still gets the real cause. The app shows these directly as the status line.
- **`RenderErrorKind`** (`NoGpu`, `DeviceLost`, `OutOfMemory`, `Cancelled`, `Other`) is set where the failure is known: no adapter or device means NoGpu; a failed wait or readback means DeviceLost; an allocation failure caught by a wgpu OutOfMemory error scope around each tile means OutOfMemory. Real GPU loss and GPU out-of-memory could not be reproduced here, so those two paths are covered only by the message tests, not by a real failure. A readback failure can in principle also be an out-of-memory; it is reported as the GPU not responding.
- **Write to a temporary name, then rename.** Every image is written as `<name>.part` beside its final name and renamed when complete (rename replaces an existing file on Windows and Unix); on any error the `.part` file is removed and an earlier file of the same name is untouched. A cancel after some files finished still removes those, as before. EXR now names its format explicitly, because `image` picks it from the extension.
- **Ask before overwriting.** `existing_outputs(dir, options)` lists the files this export would replace (only the kinds being exported). The app shows "Replace existing files?" (Replace / Cancel) listing up to eight names. Alternative: always overwrite silently (the old behavior), rejected by the issue.
- **No window crash.** If the export thread ends without a result (a panic in a driver, say) the UI says so and unlocks Export instead of staying stuck. If the window cannot open at all (no GPU or driver), `main` shows a message dialog with the likely cause. The app's remaining `expect` is the embedded icon, which a test already decodes.
- **Not done:** the Preview losing its device mid-session (egui/wgpu handle or panic); a pre-flight memory estimate before a big export.
- **Tests:** `export.rs` for no `.part` files after success, an earlier file surviving a failed export (mutation-checked: writing straight to the final name fails it), `existing_outputs`, `user_message` per I/O kind and exporting into a path that is a file; `lib.rs` for the render error kinds. The overwrite dialog, the status text and the startup dialog are GUI glue and were not triggered by hand.

## Preview performance (issue #18)

- **Measured** with `cargo run --release -p spacegen-render --example bench_preview` (new): one sky pass at Preview size, submit to GPU-idle, median of 20 frames, `Bgra8UnormSrgb` target, N Layers of one kind (same settings, defaults). Windows 11, DirectX 12. Times in milliseconds:

  | | 1 Layer | 8 Layers | 16 Layers |
  |---|---|---|---|
  | **NVIDIA RTX 5070 Ti** (discrete), 1080p star fields | 0.3 | 1.7 | 3.4 |
  | 1080p nebulae | 0.4 | 3.0 | 6.0 |
  | 1080p galaxy bands | 0.6 | 4.1 | 8.2 |
  | 1080p suns | 0.2 | 0.8 | 1.6 |
  | 4K star fields | 0.9 | 6.5 | 13.1 |
  | 4K nebulae | 1.5 | 12.1 | 24.1 |
  | 4K galaxy bands | 2.0 | 16.4 | 32.7 |
  | 4K suns | 0.5 | 3.2 | 6.1 |
  | **AMD Radeon Graphics** (integrated), 1080p star fields | 9.7 | 73 | 146 |
  | 1080p nebulae | 26.8 | 209 | 419 |
  | 1080p galaxy bands | 39.8 | 314 | 648 |
  | 1080p suns | 3.6 | 26 | 52 |
  | 4K star fields | 35.7 | 273 | 543 |
  | 4K nebulae | 106 | 836 | 1673 |
  | 4K galaxy bands | 158 | 1253 | (GPU reset) |

  Cost is linear in Layers and in pixels. The discrete GPU manages even 16 galaxy bands at 4K at about 30 frames a second. The integrated GPU does not: 8 star fields at 1080p are 14 frames a second, 16 nebulae at 1080p about 2, and 16 galaxy bands at 4K ran past Windows' two-second GPU timeout and reset the device (the 0.00 ms rows at the end of the raw run).
- **So the Preview adapts** (the issue's "if needed"). `QualityGovernor` (`quality.rs`) picks a scale from 0.25 to 1.0 of the Preview's width and height from the measured frame time while the user is interacting: cost follows pixel count, so the scale that hits the 33 ms target is the current scale times the square root of target over frame time. It shrinks as soon as a frame is slower than the target and grows back only when a frame took under 70 % of the target (and then to 90 % of the size that would exactly hit it), so it does not flip back and forth. Alternative: a fixed low resolution while dragging, rejected because the discrete GPU needs none and the slow case varies by 100 times between machines. A fast machine never leaves full size.
- **Sharp when you pause.** After 0.4 s without input (pointer movement, a button down, scroll, key) the next frame is full size; a repaint is requested so that frame happens even when nothing else changes. On the integrated GPU that one frame can still take hundreds of milliseconds; it does not repeat unless input resumes.
- **How it draws.** `ScaledSky` (`scaled_sky.rs`) renders the sky into an offscreen texture of the reduced size (in the Preview's `prepare` step) and stretches it over the panel with a linear-filtered blit; the sky's `pixel_angle` uses the reduced pixel size so stars keep their on-screen size. Tested headlessly on the software adapter: a half-size sky stretched to the target differs from the direct render by a mean of 0.0017 (of 1.0); a vertically flipped blit scores 0.043, which fails the 0.01 limit (checked).
- **Not verified by eye:** the reduced Preview was not looked at in the running window (the app was launched, not driven), and the governor's constants (33 ms, 0.7, 0.9, 0.4 s) are first guesses without a slow machine to tune them on. The measured frame time is egui's time between frames, which on a 60 Hz display includes waiting for the display, so a frame that really takes 5 ms reads as 16 ms; that is still under the grow threshold.
- **The 16-Layer cap is shown.** The Layer list says "N of 16 visible Layers"; at 16 the Add and Copy buttons are disabled with a hover explanation, and over 16 (more Layers shown, or a file that has them) a warning says how many of the top visible Layers are not drawn. Only visible Layers count, matching the renderer. `layer_cap::cap_status` is unit-tested.

## Core crate split into modules (issue #19)

- `spacegen-core/src/lib.rs` (951 lines) is now a list of modules and `pub use` lines; the code moved without edits except for visibility. Modules: `seed` (Seed, LayerId, per-Layer seed mixing), `settings` (the four settings structs and their 0.0 to 1.0 validation), `layer` (LayerKind, BlendMode, Layer and the private settings enum), `project` (Project, edits, JSON load and save, format version), `error` (EditError, ProjectError), `history` (undo and redo, drag merging), `engine` (Face, image and HDRI formats, EngineConvention file names), `color` (the deterministic color picker for Randomize and re-roll).
- **Public API unchanged:** every public item is re-exported at the crate root under its old name, so no other crate and no test changed. Verified by building and running the whole workspace untouched.
- **Visibility:** what other modules needed became `pub(crate)` (Layer's fields, `LayerSettings`, each settings `validate`, `Layer::recolor`, `ColorPicker`), and nothing is `pub` that was not before. `ColorPicker` got a `new` constructor rather than a public field.
- **Cohesion over size:** `Project` stays one module with its edit methods (about 240 lines) rather than splitting edits from serialization, because both need its private fields and splitting would widen them.
