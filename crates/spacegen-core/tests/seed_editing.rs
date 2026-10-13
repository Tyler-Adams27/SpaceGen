use spacegen_core::{Project, Seed};

#[test]
fn set_seed_replaces_the_seed_and_accepts_every_u64() {
    let mut project = Project::new(Seed(5));

    for value in [0, 9, u64::MAX] {
        project.set_seed(Seed(value));
        assert_eq!(project.seed(), Seed(value));
    }
}
