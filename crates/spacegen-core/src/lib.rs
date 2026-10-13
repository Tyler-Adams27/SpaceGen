#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Seed(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LayerId(pub u64);

impl Seed {
    pub fn layer_seed(self, layer: LayerId) -> Seed {
        const GAMMA: u64 = 0x9E3779B97F4A7C15;
        let state = self
            .0
            .wrapping_add(layer.0.wrapping_mul(GAMMA))
            .wrapping_add(GAMMA);
        let mut z = state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        Seed(z ^ (z >> 31))
    }
}
