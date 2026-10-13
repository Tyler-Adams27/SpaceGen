use spacegen_core::{LayerId, Seed};

// Expected values are the published SplitMix64 reference outputs for state 0
// (the 1st, 2nd and 3rd generated numbers), not recomputed here.

#[test]
fn first_layer_of_seed_zero_gets_the_first_splitmix64_reference_output() {
    assert_eq!(
        Seed(0).layer_seed(LayerId(0)),
        Seed(0xE220A8397B1DCDAF)
    );
}

#[test]
fn second_layer_of_seed_zero_gets_the_second_splitmix64_reference_output() {
    assert_eq!(
        Seed(0).layer_seed(LayerId(1)),
        Seed(0x6E789E6AA1B965F4)
    );
}

#[test]
fn different_layers_of_the_same_seed_get_different_layer_seeds() {
    let seed = Seed(42);
    assert_ne!(seed.layer_seed(LayerId(1)), seed.layer_seed(LayerId(2)));
}

#[test]
fn the_same_layer_under_different_seeds_gets_different_layer_seeds() {
    let layer = LayerId(7);
    assert_ne!(Seed(1).layer_seed(layer), Seed(2).layer_seed(layer));
}
