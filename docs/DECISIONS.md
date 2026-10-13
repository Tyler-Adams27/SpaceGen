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
