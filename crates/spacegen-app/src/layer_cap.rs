//! The sky draws at most `MAX_LAYERS` visible Layers (the bottom ones); this says where a Project
//! stands against that limit so the UI can show it instead of ignoring the extra Layers silently.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapStatus {
    /// Room for at least one more visible Layer.
    Room,
    /// Exactly at the limit: another visible Layer would not be drawn.
    Full,
    /// Over the limit by this many: the top ones are not drawn.
    Over(usize),
}

pub fn cap_status(visible_layers: usize, max: usize) -> CapStatus {
    match visible_layers.cmp(&max) {
        std::cmp::Ordering::Less => CapStatus::Room,
        std::cmp::Ordering::Equal => CapStatus::Full,
        std::cmp::Ordering::Greater => CapStatus::Over(visible_layers - max),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn below_the_limit_there_is_room() {
        assert_eq!(cap_status(0, 16), CapStatus::Room);
        assert_eq!(cap_status(15, 16), CapStatus::Room);
    }

    #[test]
    fn exactly_at_the_limit_is_full() {
        assert_eq!(cap_status(16, 16), CapStatus::Full);
    }

    #[test]
    fn beyond_the_limit_says_how_many_are_not_drawn() {
        assert_eq!(cap_status(17, 16), CapStatus::Over(1));
        assert_eq!(cap_status(20, 16), CapStatus::Over(4));
    }
}
