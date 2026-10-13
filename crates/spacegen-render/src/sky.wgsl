// Procedural sky: one fullscreen pass that sums every star field Layer.
// The Preview, the Face renders and the equirectangular render all use this
// shader, so what you see is what you export.

const MAX_LAYERS: u32 = 16u;
const TAU: f32 = 6.2831853;
const PI: f32 = 3.1415927;

// ---- Look constants: tune the sky here -------------------------------------
const STAR_GRID: f32 = 60.0;          // grid cells per unit direction (star spacing)
const STAR_PRESENCE_SCALE: f32 = 0.45; // chance a cell holds a star at density 1.0
const STAR_MIN_RADIUS: f32 = 0.0012;  // angular radius (radians) at size 0.0
const STAR_MAX_RADIUS: f32 = 0.0060;  // angular radius (radians) at size 1.0
const STAR_PIXEL_FLOOR: f32 = 0.9;    // never smaller than this many pixels
const STAR_BRIGHTNESS_BASE: f32 = 0.08;
const STAR_BRIGHTNESS_RANGE: f32 = 4.0;
const STAR_BRIGHTNESS_SKEW: f32 = 9.0; // higher = fewer bright stars
// ----------------------------------------------------------------------------

struct Layer {
    seed: vec2<u32>,
    blend: u32,          // 0 = additive, 1 = screen
    pad: u32,
    params: vec4<f32>,   // density, brightness, size, color_variation
    tint_opacity: vec4<f32>,
}

struct Globals {
    origin: vec4<f32>,
    right: vec4<f32>,
    down: vec4<f32>,
    info: vec4<u32>,     // layer count, mode (0 = basis, 1 = equirectangular)
    pixel: vec4<f32>,    // angular size of one pixel in radians
    layers: array<Layer, 16>,
}

@group(0) @binding(0) var<uniform> g: Globals;

struct VsOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> VsOut {
    var corners = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );
    let xy = corners[index];
    var out: VsOut;
    out.position = vec4<f32>(xy, 0.0, 1.0);
    out.uv = vec2<f32>((xy.x + 1.0) * 0.5, (1.0 - xy.y) * 0.5);
    return out;
}

fn view_direction(uv: vec2<f32>) -> vec3<f32> {
    if (g.info.y == 0u) {
        return normalize(g.origin.xyz + uv.x * g.right.xyz + uv.y * g.down.xyz);
    }
    // Equirectangular: the image centre looks toward +Z, +X is a quarter turn right.
    let lon = uv.x * TAU;
    let theta = uv.y * PI;
    return vec3<f32>(sin(lon) * sin(theta), cos(theta), -cos(lon) * sin(theta));
}

fn pcg(v: u32) -> u32 {
    let s = v * 747796405u + 2891336453u;
    let w = ((s >> ((s >> 28u) + 4u)) ^ s) * 277803737u;
    return (w >> 22u) ^ w;
}

fn rand01(h: u32) -> f32 {
    return f32(h >> 8u) / 16777216.0;
}

fn star_color(t: f32) -> vec3<f32> {
    let blue = vec3<f32>(0.65, 0.78, 1.0);
    let white = vec3<f32>(1.0, 0.97, 0.92);
    let yellow = vec3<f32>(1.0, 0.88, 0.6);
    let orange = vec3<f32>(1.0, 0.62, 0.38);
    if (t < 0.35) {
        return mix(blue, white, t / 0.35);
    }
    if (t < 0.7) {
        return mix(white, yellow, (t - 0.35) / 0.35);
    }
    return mix(yellow, orange, (t - 0.7) / 0.3);
}

fn star_layer(d: vec3<f32>, layer: Layer) -> vec3<f32> {
    let density = layer.params.x;
    let brightness = layer.params.y;
    let size = layer.params.z;
    let variation = layer.params.w;
    if (density <= 0.0 || brightness <= 0.0) {
        return vec3<f32>(0.0);
    }

    let presence = density * density * STAR_PRESENCE_SCALE;
    let radius = max(mix(STAR_MIN_RADIUS, STAR_MAX_RADIUS, size), g.pixel.x * STAR_PIXEL_FLOOR);
    let base = vec3<i32>(floor(d * STAR_GRID));
    var light = vec3<f32>(0.0);

    for (var dx = -1; dx <= 1; dx++) {
        for (var dy = -1; dy <= 1; dy++) {
            for (var dz = -1; dz <= 1; dz++) {
                let cell = base + vec3<i32>(dx, dy, dz);
                var h = pcg(bitcast<u32>(cell.x) ^ layer.seed.x);
                h = pcg(h ^ bitcast<u32>(cell.y) ^ layer.seed.y);
                h = pcg(h ^ bitcast<u32>(cell.z));
                if (rand01(h) >= presence) {
                    continue;
                }
                let h1 = pcg(h ^ 0x9E3779B9u);
                let h2 = pcg(h1);
                let h3 = pcg(h2);
                let h4 = pcg(h3);
                let h5 = pcg(h4);
                let point = vec3<f32>(cell) + vec3<f32>(rand01(h1), rand01(h2), rand01(h3));
                let len = length(point);
                // Only cells the sphere passes through hold stars, so a star always
                // sits within the neighbouring cells of every direction near it.
                if (abs(len - STAR_GRID) > 0.5) {
                    continue;
                }
                let angle = length(d - point / len);
                if (angle > radius * 3.0) {
                    continue;
                }
                let fall = exp(-(angle * angle) / (radius * radius));
                let peak = brightness * (STAR_BRIGHTNESS_BASE
                    + STAR_BRIGHTNESS_RANGE * pow(rand01(h4), STAR_BRIGHTNESS_SKEW));
                let color = mix(vec3<f32>(1.0), star_color(rand01(h5)), variation);
                light += color * peak * fall;
            }
        }
    }
    return light * layer.tint_opacity.xyz;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let d = view_direction(in.uv);
    var color = vec3<f32>(0.0);
    let count = min(g.info.x, MAX_LAYERS);
    for (var i = 0u; i < count; i++) {
        let layer = g.layers[i];
        let light = star_layer(d, layer) * layer.tint_opacity.w;
        if (layer.blend == 1u) {
            color = vec3<f32>(1.0) - (vec3<f32>(1.0) - color) * (vec3<f32>(1.0) - clamp(light, vec3<f32>(0.0), vec3<f32>(1.0)));
        } else {
            color = color + light;
        }
    }
    return vec4<f32>(color, 1.0);
}
