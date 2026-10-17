/// A small deterministic random color source (SplitMix64) for `Project::randomize_colors`.
pub(crate) struct ColorPicker(u64);

impl ColorPicker {
    pub(crate) fn new(state: u64) -> ColorPicker {
        ColorPicker(state)
    }

    /// The next number in 0.0 to 1.0.
    pub(crate) fn next(&mut self) -> f32 {
        self.0 = self.0.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        ((z ^ (z >> 31)) >> 40) as f32 / (1u64 << 24) as f32
    }

    pub(crate) fn pastel(&mut self) -> [f32; 3] {
        let (hue, saturation) = (self.next(), self.next() * 0.3);
        hsv_to_rgb(hue, saturation, 1.0)
    }

    pub(crate) fn vivid(&mut self, hue: f32) -> [f32; 3] {
        let (saturation, value) = (0.55 + self.next() * 0.45, 0.8 + self.next() * 0.2);
        hsv_to_rgb(hue, saturation, value)
    }

    pub(crate) fn vivid_pair(&mut self) -> ([f32; 3], [f32; 3]) {
        let hue = self.next();
        let second = (hue + 0.15 + self.next() * 0.35).fract();
        (self.vivid(hue), self.vivid(second))
    }
}

/// Hue, saturation and value, each 0.0 to 1.0, as red, green and blue.
fn hsv_to_rgb(hue: f32, saturation: f32, value: f32) -> [f32; 3] {
    let sector = hue.fract() * 6.0;
    let (index, fraction) = (sector.floor() as u32 % 6, sector.fract());
    let low = value * (1.0 - saturation);
    let falling = value * (1.0 - saturation * fraction);
    let rising = value * (1.0 - saturation * (1.0 - fraction));
    match index {
        0 => [value, rising, low],
        1 => [falling, value, low],
        2 => [low, value, rising],
        3 => [low, falling, value],
        4 => [rising, low, value],
        _ => [value, low, falling],
    }
}
