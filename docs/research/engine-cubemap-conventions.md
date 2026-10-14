# Engine cubemap / skybox import conventions

Researched 2026-10-13 from primary sources only. Anything not confirmed by a primary source is marked **UNCONFIRMED**.
Unreal pages cited are UE 4.27 docs unless noted; UE5 behaviour may differ.

## Summary table (neutral face names)

| Face | Unity (6 Sided material slot) | Unreal (DDS / cubemap face order) | Godot (Cubemap import layer) | Blender |
|---|---|---|---|---|
| +X | `Left [+X]`; flip/rotation UNCONFIRMED | "Positive X" (face 0); NVIDIA-plugin source image rotated 90 CCW; UE5 UNCONFIRMED | `X+` (layer 0); no flip applied by importer | No six-face import (UNCONFIRMED) |
| -X | `Right [-X]`; UNCONFIRMED | "Negative X" (face 1); plugin: rotated 90 CW; UE5 UNCONFIRMED | `X-` (layer 1); no flip | n/a |
| +Y | `Up [+Y]`; UNCONFIRMED | "Positive Y" (face 2); plugin: rotated 180; UE5 UNCONFIRMED | `Y+` (layer 2, up); no flip | n/a |
| -Y | `Down [-Y]`; UNCONFIRMED | "Negative Y" (face 3); plugin: no rotation; UE5 UNCONFIRMED | `Y-` (layer 3); no flip | n/a |
| +Z | `Front [+Z]`; UNCONFIRMED | "Positive Z" (face 4); plugin: "side that must align with positive Y at the top"; UE5 UNCONFIRMED | `Z+` (layer 4); no flip | n/a |
| -Z | `Back [-Z]`; UNCONFIRMED | "Negative Z" (face 5); same plugin note; UE5 UNCONFIRMED | `Z-` (layer 5, "forward"); no flip | n/a |

Notes on the table:
- Unity `_FrontTex` etc.: the manual gives only the labels (Front/Back/Left/Right/Up/Down), not shader property IDs. The `_XxxTex` names are **UNCONFIRMED** from a primary source in this research.
- Unity labels are counter-intuitive: the slot labelled "Left" is +X, which the manual says sits to the *right* of the default camera.
- Godot "no flip": the importer slices the atlas row-major and passes the slices unchanged as layers 0..5 (source below). The orientation each face image must have is set by the GL/Vulkan cubemap table below; that Godot defers to it is **UNCONFIRMED** (Godot docs do not say).
- Blender: the Environment Texture node documents only Equirectangular and Mirror Ball projections; no cube-face input is documented.

## Coordinate systems

- Unity: Y-up (skybox Rotation is "around the positive y-axis"); +Z is in front of the default camera. https://docs.unity3d.com/Manual/shader-skybox-6sided.html . Handedness: UNCONFIRMED (not retrieved).
- Unreal: left-handed, Z-up, +X forward, +Y right. https://dev.epicgames.com/documentation/en-us/unreal-engine/coordinate-system-and-spaces-in-unreal-engine
- Godot: Y+ up, Z- forward. https://docs.godotengine.org/en/stable/tutorials/assets_pipeline/importing_images.html . Handedness: UNCONFIRMED (not fetched).
- Blender: Z-up ("environment with the Z axis pointing up"). https://docs.blender.org/manual/en/latest/render/shader_nodes/textures/environment.html . Handedness: UNCONFIRMED.

## Khronos reference (graphics API cubemap convention)

Vulkan cube map face selection, https://registry.khronos.org/vulkan/specs/latest/html/vkspec.html (section "Cube Map Face Selection", Table 32):

| Face (layer) | sc | tc |
|---|---|---|
| +X (0) | -rz | -ry |
| -X (1) | +rz | -ry |
| +Y (2) | +rx | +rz |
| -Y (3) | +rx | -rz |
| +Z (4) | +rx | -ry |
| -Z (5) | -rx | -ry |

Layer order is +X, -X, +Y, -Y, +Z, -Z. This is the spec-level convention; which engines use it directly is not stated by their docs except where noted below. The OpenGL spec carries an equivalent table (https://registry.khronos.org/OpenGL/specs/gl/glspec46.core.pdf), not parsed here.

## Per engine

### Unity
- Six separate images: `Skybox/6 Sided` material, slots Front [+Z], Back [-Z], Left [+X], Right [-X], Up [+Y], Down [-Y]; Rotation about +Y. https://docs.unity3d.com/Manual/shader-skybox-6sided.html (HDR textures recommended). Works in Built-in and URP, not HDRP: https://docs.unity3d.com/6000.0/Documentation/Manual/skybox-shaders.html
- Per-face flips/rotations for the 6-sided slots: UNCONFIRMED (manual only says it provides a net layout image).
- Single image, cubemap import: Texture Shape Cube, Mapping Auto / 6 Frames Layout / Latitude Longitude / Mirrored Ball. Six images "in one of the standard cubemap layouts: cross, or sequence (+x -x +y -y +z -z)", horizontal or vertical. https://docs.unity3d.com/Manual/texture-type-cubemap.html . Cross layout face placement and per-face orientation: UNCONFIRMED.
- Equirectangular: Panoramic skybox takes a "latitude-longitude (cylindrical)" 2D texture (Image Type 180 puts the "peak" toward +Z; 360 wraps fully). https://docs.unity3d.com/Manual/shader-skybox-panoramic.html and https://docs.unity3d.com/Manual/skybox-shaders.html . u direction / seam / centre direction for 360: UNCONFIRMED (shader source not obtained).
- HDR file formats accepted (.exr/.hdr): UNCONFIRMED (pages say textures "should use HDR" but do not list formats).

### Unreal Engine
- Cubemap face order (viewer, matches DDS): Positive X, Negative X, Positive Y, Negative Y, Positive Z, Negative Z. https://dev.epicgames.com/documentation/en-us/unreal-engine/texture-asset-editor-in-unreal-engine
- Six images: Epic documents only the NVIDIA Photoshop plug-in route: six images side by side in one document in order +X,-X,+Y,-Y,+Z,-Z, with the per-face rotations in the table (X+ 90 CCW, X- 90 CW, Y+ 180, Y- none, Z+/Z- the Y-positive side at top). The page does not mention flips. https://dev.epicgames.com/documentation/en-us/unreal-engine/creating-cubemaps?application_version=4.27 . Direct import of six PNGs or a cross/strip into a Texture Cube: UNCONFIRMED.
- Supported import formats: .bmp .float .pcx .png .psd .tga .jpg .exr .dds .hdr; `.dds` = cubemap, `.hdr` = "Cubemap Texture (LongLat unwrap)". https://dev.epicgames.com/documentation/en-us/unreal-engine/texture-import-guide?application_version=4.27 . Texture Asset Editor page: "Import HDR either imports the image as a 2D texture or a LatLong Cubemap" and lists "cubemaps (.exr)": https://dev.epicgames.com/documentation/en-us/unreal-engine/texture-asset-editor-in-unreal-engine . Whether .exr imports as a long-lat cubemap: UNCONFIRMED.
- Equirectangular orientation (centre direction, u/v direction): UNCONFIRMED (docs say only "longitude/latitude" unwrap).

### Godot
- Import type Cubemap: "expected image order is X+, X-, Y+, Y-, Z+, Z-" in Godot's coordinate system (Y+ up, Z- forward). Templates 2x3 (default), 3x2, 1x6, 6x1. https://docs.godotengine.org/en/stable/tutorials/assets_pipeline/importing_images.html
- Source confirms: image split into hslices x vslices cells, row-major (rows top to bottom, columns left to right), each cell becomes one layer in that order; no flip/rotate. https://github.com/godotengine/godot/blob/master/editor/import/resource_importer_layered_texture.cpp (lines ~307-380). So the 2x3 default = row 0: +X,-X; row 1: +Y,-Y; row 2: +Z,-Z (derived from that source). A cross layout is not offered (only the four arrangements).
- Six separate files: build a Cubemap resource from six Images in layer order; exactly 6 images of equal size/format required. https://github.com/godotengine/godot/blob/master/scene/resources/image_texture.cpp
- Equirectangular (PanoramaSkyMaterial): must be equirectangular; `.hdr` and `.exr` supported (HDR strongly recommended). https://docs.godotengine.org/en/stable/classes/class_panoramaskymaterial.html and the importing_images page above (also documents HDR Clamp Exposure, HDR as sRGB).
- **Confirmed in practice (user, 2026-10-14):** SpaceGen's exported panorama in a `PanoramaSkyMaterial` shows no seam and is oriented correctly. Six-Face import into Godot is still unchecked.
- **Confirmed in practice (user, 2026-10-14):** the six Faces, written in the Khronos/Vulkan orientation with only file names varying, tile correctly in Unity, Unreal and Godot with no flip or rotation applied. This supersedes the "unchecked" note above; the per-face UNCONFIRMED cells in the table describe what the docs fail to state, not a problem observed in use.
- Mapping from sky shader source: `uv = (atan2(dir.x, -dir.z) wrapped to [0,2pi) / 2pi, acos(dir.y) / pi)`. So u=0 at -Z, u=0.25 at +X, u=0.5 at +Z (image centre looks toward +Z), u=0.75 at -X; v=0 at +Y (top row is up). https://github.com/godotengine/godot/blob/master/servers/rendering/renderer_rd/shaders/environment/sky.glsl (lines ~226-232). Master branch code; older releases may differ.

### Blender
- World / Environment Texture node projections: Equirectangular and Mirror Ball only; unconnected Vector maps "as an environment with the Z axis pointing up". https://docs.blender.org/manual/en/latest/render/shader_nodes/textures/environment.html . No six-face / cross / strip cubemap input documented. (Cycles has an "equiangular cubemap face" camera panorama, which is a camera type, not a world texture input: https://github.com/blender/blender/blob/main/intern/cycles/kernel/camera/projection.h)
- Equirectangular mapping from Cycles source: `u = 0.5 - atan2(dir.y, dir.x)/(2pi)`, `v = 1 - acos(dir.z)/pi` (`direction_to_equirectangular`, range (-2pi, pi, -pi, pi)); the SVM environment node calls it (https://github.com/blender/blender/blob/main/intern/cycles/kernel/svm/tex_coord.h line ~194). So u=0.5 (image centre) at +X, u=0.25 at +Y, u=0.75 at -Y, u=0/1 at -X; v=1 (Blender image v is bottom-up, so top of image) at +Z. Source: https://github.com/blender/blender/blob/main/intern/cycles/kernel/camera/projection.h (main branch, 2026-10-13). The manual does not state the centre direction (UNCONFIRMED in docs). The tex_coord.h hit is the generic coordinate function; that the Environment Texture node uses exactly this path was inferred from the call site, not read end to end.
- Image formats: the node page uses an HDR image as example and defers to shared image settings; no explicit .exr/.hdr list found there. Format acceptance UNCONFIRMED (Blender image-format docs not fetched).

## Resolved 2026-10-14 (issue #5)
- Unity HDR formats: **confirmed.** The import list is "BMP, EXR, GIF, HDR, IFF, JPG, PICT, PNG, PSD, TGA, TIFF, SVG". https://docs.unity3d.com/Manual/ImportingTextures.html
- Blender image formats: **confirmed.** OpenEXR (float 16/32, `.exr`) and Radiance HDR (float, `.hdr`) are in "the list of image file formats supported internally by Blender". https://docs.blender.org/manual/en/latest/files/media/image_formats.html
- Godot handedness: **confirmed right-handed**, Y up ("Godot uses a right-handed coordinate system"). The page does not state the sign of the forward axis; Z- forward rests on the importing_images page cited above. https://docs.godotengine.org/en/stable/tutorials/assets_pipeline/importing_images.html and https://docs.godotengine.org/en/stable/tutorials/3d/introduction_to_3d.html
- Blender: the manual says only "right-angled Cartesian ... Z axis pointing upwards" and never uses the word right-handed in the pages checked. Handedness stays undocumented there.
- Unreal `.hdr`: confirmed as "Cubemap Texture (LongLat unwrap)" (4.27 import guide, cited above). A UE5 search found the same wording; `.exr` as a long-lat cubemap is still not stated in the docs.
- Unity cross layout: the cubemap page lists "cross, or sequence (+x -x +y -y +z -z)" and "horizontal or vertical" and says nothing about face placement or orientation. Not needed: SpaceGen exports six separate Faces.
- Settled by observation (user, 2026-10-14): Faces need no per-engine flip or rotation in Unity, Unreal or Godot, which covers the per-face flip cells, "UE5 six-face import and flips", and "whether Godot face orientation equals the Khronos table" (it works, so it does, in effect).
- Godot panorama mapping re-read from `sky.glsl` on master: `atan2(x, -z)` and `acos(y)`, then divided by (2pi, pi), matching the mapping above.

## Open items (UNCONFIRMED, no effect on any export seen so far)
- Unity: panoramic centre/u direction, `_XxxTex` shader property IDs, handedness.
- Unreal: `.exr` as a long-lat cubemap, equirect centre/u direction.
- Blender: handedness, doc-level statement of the equirect centre. Six-face import: none documented.
- Panorama use in Unity and Unreal has not been tried.
