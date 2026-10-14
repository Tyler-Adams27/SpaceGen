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
const STAR_MAX_RADIUS: f32 = 0.0055;  // angular radius (radians) at size 1.0; keep <= 1 / (3 * STAR_GRID)
const STAR_PIXEL_FLOOR: f32 = 0.9;    // never smaller than this many pixels
const STAR_BRIGHTNESS_BASE: f32 = 0.08;
const STAR_BRIGHTNESS_RANGE: f32 = 4.0;
const STAR_BRIGHTNESS_SKEW: f32 = 9.0; // higher = fewer bright stars
const NEBULA_MIN_FREQUENCY: f32 = 1.2;  // noise features across the sky at scale 0.0
const NEBULA_MAX_FREQUENCY: f32 = 7.0;  // ... and at scale 1.0
const NEBULA_THRESHOLD_FULL: f32 = 0.30; // cloud cut-off at coverage 1.0 (lower = more cloud)
const NEBULA_THRESHOLD_NONE: f32 = 0.85; // cloud cut-off at coverage 0.0
const NEBULA_SOFTNESS: f32 = 0.30;       // how gradually cloud fades in above the cut-off
const NEBULA_GAIN: f32 = 0.9;            // overall nebula brightness at brightness 1.0
const BAND_MIN_SIGMA: f32 = 0.08;     // half-width of the band (sine of its angle off the centre line) at width 0.0
const BAND_MAX_SIGMA: f32 = 0.55;     // ... and at width 1.0
const BAND_CORE_FRACTION: f32 = 0.45; // the bright core is this fraction of the band width
const BAND_DUST_FRACTION: f32 = 0.30; // dust lanes sit in this fraction of the band width
const BAND_DUST_DEPTH: f32 = 0.92;    // how much light a full-strength dust lane removes
const BAND_GAIN: f32 = 0.65;          // overall band brightness at brightness 1.0
const SUN_MIN_RADIUS: f32 = 0.012;    // angular radius (radians) of the disc at size 0.0
const SUN_MAX_RADIUS: f32 = 0.16;     // ... and at size 1.0
const SUN_PIXEL_FLOOR: f32 = 0.9;     // the disc is never smaller than this many pixels
const SUN_PEAK: f32 = 8.0;            // disc intensity at brightness 1.0; above 1.0 so the EXR keeps it
const SUN_HALO_GAIN: f32 = 2.5;       // halo intensity at the disc edge at glare 1.0
const SUN_HALO_REACH: f32 = 6.0;      // halo fades by 1/e over this many disc radii
const SUN_GLARE_GAIN: f32 = 0.25;     // the faint wide glare, at glare 1.0
// ----------------------------------------------------------------------------

struct Layer {
    seed: vec2<u32>,
    blend: u32,          // 0 = additive, 1 = screen
    kind: u32,           // 0 = star field, 1 = nebula, 2 = galaxy band, 3 = sun
    params: vec4<f32>,   // stars: density, brightness, size, color variety; nebula: coverage, scale, brightness; galaxy band: tilt, width, brightness, dust
    color_a: vec4<f32>,  // stars: tint; nebula, galaxy band: first color; w = Layer opacity
    color_b: vec4<f32>,  // nebula, galaxy band: second color; sun: x = glare
}

struct Globals {
    origin: vec4<f32>,
    right: vec4<f32>,
    down: vec4<f32>,
    info: vec4<u32>,     // layer count, mode (0 = basis, 1 = equirectangular)
    pixel: vec4<f32>,    // angular size of one pixel in radians
    band: vec4<f32>,     // the slice of the image this pass covers: v start, v end
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
    // The three-cell search only finds a star within one grid cell of its centre, so the
    // visible part (3 radii) must fit in a cell, however coarse the render is.
    let radius = min(
        max(mix(STAR_MIN_RADIUS, STAR_MAX_RADIUS, size), g.pixel.x * STAR_PIXEL_FLOOR),
        1.0 / (3.0 * STAR_GRID),
    );
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
    return light * layer.color_a.xyz;
}

fn lattice(p: vec3<i32>, seed: vec2<u32>) -> f32 {
    var h = pcg(bitcast<u32>(p.x) ^ seed.x);
    h = pcg(h ^ bitcast<u32>(p.y) ^ seed.y);
    h = pcg(h ^ bitcast<u32>(p.z));
    return rand01(h);
}

fn value_noise(p: vec3<f32>, seed: vec2<u32>) -> f32 {
    let cell = vec3<i32>(floor(p));
    let f = fract(p);
    let s = f * f * (3.0 - 2.0 * f);
    let c000 = lattice(cell, seed);
    let c100 = lattice(cell + vec3<i32>(1, 0, 0), seed);
    let c010 = lattice(cell + vec3<i32>(0, 1, 0), seed);
    let c110 = lattice(cell + vec3<i32>(1, 1, 0), seed);
    let c001 = lattice(cell + vec3<i32>(0, 0, 1), seed);
    let c101 = lattice(cell + vec3<i32>(1, 0, 1), seed);
    let c011 = lattice(cell + vec3<i32>(0, 1, 1), seed);
    let c111 = lattice(cell + vec3<i32>(1, 1, 1), seed);
    let x00 = mix(c000, c100, s.x);
    let x10 = mix(c010, c110, s.x);
    let x01 = mix(c001, c101, s.x);
    let x11 = mix(c011, c111, s.x);
    return mix(mix(x00, x10, s.y), mix(x01, x11, s.y), s.z);
}

fn fbm(p: vec3<f32>, seed: vec2<u32>) -> f32 {
    var sum = 0.0;
    var amplitude = 0.5;
    var q = p;
    for (var octave = 0; octave < 6; octave++) {
        sum += amplitude * value_noise(q, seed);
        q = q * 2.03 + vec3<f32>(17.1, 9.7, 31.3);
        amplitude *= 0.5;
    }
    return sum;
}

fn nebula_layer(d: vec3<f32>, layer: Layer) -> vec3<f32> {
    let coverage = layer.params.x;
    let scale = layer.params.y;
    let brightness = layer.params.z;
    if (coverage <= 0.0 || brightness <= 0.0) {
        return vec3<f32>(0.0);
    }
    let p = d * mix(NEBULA_MIN_FREQUENCY, NEBULA_MAX_FREQUENCY, scale);
    let density = fbm(p, layer.seed);
    let threshold = mix(NEBULA_THRESHOLD_NONE, NEBULA_THRESHOLD_FULL, coverage);
    let cloud = smoothstep(threshold, threshold + NEBULA_SOFTNESS, density);
    // A second, slower noise field decides where the gas is color A or color B.
    let tone = fbm(p * 0.6 + vec3<f32>(5.2, 1.3, 8.1), layer.seed ^ vec2<u32>(0x68E31DA4u, 0xB5297A4Du));
    let color = mix(layer.color_a.xyz, layer.color_b.xyz, smoothstep(0.3, 0.7, tone));
    return color * cloud * cloud * brightness * NEBULA_GAIN;
}

fn galaxy_band_layer(d: vec3<f32>, layer: Layer) -> vec3<f32> {
    let tilt = layer.params.x;
    let width = layer.params.y;
    let brightness = layer.params.z;
    let dust = layer.params.w;
    if (brightness <= 0.0) {
        return vec3<f32>(0.0);
    }
    // The band is a great circle; tilt swings its plane from the horizon up toward vertical.
    let angle = tilt * 0.5 * PI;
    let normal = vec3<f32>(0.0, cos(angle), sin(angle));
    let offset = dot(d, normal);
    let sigma = mix(BAND_MIN_SIGMA, BAND_MAX_SIGMA, width);
    let glow = exp(-(offset * offset) / (sigma * sigma));
    // Clumps of brighter and thinner band, plus a fine grain of unresolved stars.
    let clumps = fbm(d * 3.0, layer.seed);
    let grain = fbm(d * 40.0, layer.seed ^ vec2<u32>(0x2C1B3C6Du, 0x297A2D39u));
    var light = glow * mix(0.35, 1.0, smoothstep(0.25, 0.75, clumps)) * mix(0.6, 1.0, grain);
    // Dust lanes: dark, broken-up streaks along the middle of the band.
    let lane_sigma = sigma * BAND_DUST_FRACTION;
    let lane = exp(-(offset * offset) / (lane_sigma * lane_sigma));
    let breakup = smoothstep(0.35, 0.65, fbm(d * 7.0 + vec3<f32>(3.7, 11.1, 5.3), layer.seed ^ vec2<u32>(0x1B873593u, 0xCC9E2D51u)));
    light *= 1.0 - dust * lane * breakup * BAND_DUST_DEPTH;
    let core_sigma = sigma * BAND_CORE_FRACTION;
    let core = exp(-(offset * offset) / (core_sigma * core_sigma));
    let color = mix(layer.color_b.xyz, layer.color_a.xyz, core);
    return color * light * brightness * BAND_GAIN;
}

fn sun_layer(d: vec3<f32>, layer: Layer) -> vec3<f32> {
    let brightness = layer.params.w;
    let glare = layer.color_b.x;
    if (brightness <= 0.0) {
        return vec3<f32>(0.0);
    }
    // Azimuth 0 looks toward +Z and 0.25 a quarter turn toward +X, like the panorama's centre.
    let lon = layer.params.x * TAU;
    let lat = (layer.params.y - 0.5) * PI;
    let direction = vec3<f32>(sin(lon) * cos(lat), sin(lat), cos(lon) * cos(lat));
    // The angle from the Sun, from the chord so it stays accurate right next to the disc.
    let angle = 2.0 * asin(min(length(d - direction) * 0.5, 1.0));
    let radius = max(mix(SUN_MIN_RADIUS, SUN_MAX_RADIUS, layer.params.z), g.pixel.x * SUN_PIXEL_FLOOR);
    let disc = 1.0 - smoothstep(radius * 0.85, radius, angle);
    let halo = glare * SUN_HALO_GAIN * exp(-angle / (radius * SUN_HALO_REACH));
    let wide = glare * SUN_GLARE_GAIN * pow(radius / (angle + radius), 2.0);
    return layer.color_a.xyz * (disc * SUN_PEAK + halo + wide) * brightness;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let uv = vec2<f32>(in.uv.x, mix(g.band.x, g.band.y, in.uv.y));
    let d = view_direction(uv);
    var color = vec3<f32>(0.0);
    let count = min(g.info.x, MAX_LAYERS);
    for (var i = 0u; i < count; i++) {
        let layer = g.layers[i];
        var light = vec3<f32>(0.0);
        if (layer.kind == 1u) {
            light = nebula_layer(d, layer);
        } else if (layer.kind == 2u) {
            light = galaxy_band_layer(d, layer);
        } else if (layer.kind == 3u) {
            light = sun_layer(d, layer);
        } else {
            light = star_layer(d, layer);
        }
        light = light * layer.color_a.w;
        if (layer.blend == 1u) {
            color = vec3<f32>(1.0) - (vec3<f32>(1.0) - color) * (vec3<f32>(1.0) - clamp(light, vec3<f32>(0.0), vec3<f32>(1.0)));
        } else {
            color = color + light;
        }
    }
    return vec4<f32>(color, 1.0);
}
