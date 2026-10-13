//! Where "Randomize" gets its numbers. The core crate stays deterministic, so the
//! app supplies the randomness.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use spacegen_core::Seed;

pub fn fresh_seed() -> Seed {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos() as u64);
    let count = COUNTER.fetch_add(1, Ordering::Relaxed);
    // SplitMix64 finalizer: spreads the clock and counter over all 64 bits.
    let mut z = nanos ^ count.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    Seed(z ^ (z >> 31))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn consecutive_seeds_are_all_different() {
        let seeds: std::collections::HashSet<u64> = (0..1000).map(|_| fresh_seed().0).collect();

        assert_eq!(seeds.len(), 1000);
    }
}
