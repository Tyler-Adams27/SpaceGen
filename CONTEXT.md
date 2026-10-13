# SpaceGen

A desktop application for procedurally generating space skyboxes for use in game engines and 3D tools.

## Language

**Skybox**:
The full 360° space environment the user is creating, viewed from a single point inside it.

**Seed**:
A number that, together with the Layer settings, determines the Skybox. The same Seed and settings give the same Skybox (small floating-point differences between machines are acceptable).

**Layer**:
One visual component of a Skybox, with its own parameters. Layers are stacked and combined to form the Skybox.
_Avoid_: effect, object

**Layer kind**:
The type of a Layer: star field, nebula, galaxy band, or sun. (Planets are deferred.)

**Project**:
A saved, reloadable document holding a Seed and a Layer stack.
_Avoid_: preset, scene

**Preview**:
The live, interactive view of the Skybox that updates as parameters change.

**Export**:
A high-resolution render of the Skybox written to disk, separate from the Preview.

**Face**:
One of the six square images that make up a cubemap.

**Export format**:
One of three outputs: six individual Faces (PNG or JPEG), a single full image (equirectangular, PNG or JPEG), or an HDRI (OpenEXR, with Radiance as an alternative).

**Engine convention**:
The face file-naming scheme and order a target engine expects (Unity, Unreal, Godot). Selected at Export. It covers names only: Face pixels are always written in one fixed orientation, and any flip is left to the user. Blender has no six-face import, so it is served by the full image and HDRI Export formats instead.

**Layer ID**:
A stable identifier for a Layer. Its randomness derives from the Seed plus its Layer ID, so editing one Layer never changes another.

**Blend mode**:
How a Layer combines with those beneath it: additive or screen.

## Relationships

- A **Project** has one **Seed** and an ordered stack of **Layers**.
- A **Skybox** is the result of the **Seed** and **Layers** combined.
- A **Preview** and an **Export** both render a **Skybox**; they differ in resolution and purpose.
- An **Export** produces one or more **Export formats**.
- A **Preview** and an **Export** show the same look; the Export differs only in resolution and anti-aliasing.
- The exported images, not the **Project**, are the source of truth: regeneration may differ slightly between machines.
