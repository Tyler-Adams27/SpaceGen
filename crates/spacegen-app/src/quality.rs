//! Keeps the Preview responsive: when frames are slow while the user is working, it is drawn at
//! a lower resolution and scaled up; once they stop it is drawn sharp again.

/// The frame time the Preview aims for while the user is interacting (about 30 frames a second).
pub const TARGET_FRAME_MS: f32 = 33.0;
/// The smallest scale: a quarter of the width and height (a sixteenth of the pixels).
pub const MIN_SCALE: f32 = 0.25;

/// Frames faster than this fraction of the target make room to grow.
const GROW_BELOW: f32 = 0.7;
/// A growing preview stops a little short of the size that would exactly hit the target.
const GROW_MARGIN: f32 = 0.9;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QualityGovernor {
    scale: f32,
}

impl Default for QualityGovernor {
    fn default() -> Self {
        QualityGovernor { scale: 1.0 }
    }
}

impl QualityGovernor {
    /// The scale (0.25 to 1.0 of the Preview's width and height) to draw the next frame at.
    /// `idle` is true once the user has stopped for a moment: then the Preview is drawn sharp.
    pub fn scale(&self, idle: bool) -> f32 {
        if idle { 1.0 } else { self.scale }
    }

    /// Records how long the last frame took, drawn at `drawn_scale`, while the user was working.
    pub fn record_frame(&mut self, frame_ms: f32, drawn_scale: f32) {
        // A frame costs about as much as it has pixels, so the scale that would hit the target is
        // the one drawn times the square root of how much too slow (or too fast) the frame was.
        let ideal = drawn_scale * (TARGET_FRAME_MS / frame_ms.max(0.1)).sqrt();
        if frame_ms > TARGET_FRAME_MS {
            self.scale = ideal.clamp(MIN_SCALE, 1.0);
        } else if frame_ms < TARGET_FRAME_MS * GROW_BELOW {
            // Grow only with room to spare, so the size does not flip back and forth.
            self.scale = (ideal * GROW_MARGIN).clamp(MIN_SCALE, 1.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_starts_sharp() {
        assert_eq!(QualityGovernor::default().scale(false), 1.0);
    }

    #[test]
    fn slow_frames_shrink_the_preview_in_proportion_to_how_slow_they_were() {
        let mut governor = QualityGovernor::default();

        // Cost follows the pixel count, so four times too slow needs half the width.
        governor.record_frame(TARGET_FRAME_MS * 4.0, 1.0);

        assert!((governor.scale(false) - 0.5).abs() < 0.01, "{governor:?}");
    }

    #[test]
    fn it_never_goes_below_the_minimum() {
        let mut governor = QualityGovernor::default();

        governor.record_frame(5000.0, 1.0);

        assert_eq!(governor.scale(false), MIN_SCALE);
    }

    #[test]
    fn a_stopped_user_always_gets_the_sharp_preview() {
        let mut governor = QualityGovernor::default();
        governor.record_frame(500.0, 1.0);

        assert_eq!(governor.scale(true), 1.0);
        assert!(governor.scale(false) < 1.0);
    }

    #[test]
    fn it_sharpens_again_only_when_the_larger_size_would_still_be_fast_enough() {
        let mut governor = QualityGovernor::default();
        governor.record_frame(TARGET_FRAME_MS * 4.0, 1.0);
        let reduced = governor.scale(false);

        // At half size a frame takes 33 ms: going back to full size would take 132 ms. Stay.
        governor.record_frame(TARGET_FRAME_MS, reduced);
        assert!((governor.scale(false) - reduced).abs() < 0.01);

        // Frames that took a quarter of the target leave room to grow.
        governor.record_frame(TARGET_FRAME_MS / 4.0, reduced);
        assert!(governor.scale(false) > reduced);
    }

    #[test]
    fn a_fast_machine_stays_at_full_size() {
        let mut governor = QualityGovernor::default();

        for _ in 0..10 {
            governor.record_frame(4.0, 1.0);
        }

        assert_eq!(governor.scale(false), 1.0);
    }
}
