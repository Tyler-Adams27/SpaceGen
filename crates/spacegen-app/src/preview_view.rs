//! How the Preview shows the sky: the free-look camera, or the whole sky flat as a panorama.

use eframe::egui::{Rect, vec2};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreviewMode {
    FreeLook,
    Flat,
}

/// The largest 2:1 rectangle that fits in `available`, centred in it. A panorama is twice as wide
/// as it is tall, and the shader maps the whole rectangle to 360 by 180 degrees, so any other
/// shape would stretch the sky.
pub fn panorama_rect(available: Rect) -> Rect {
    let width = available.width().min(available.height() * 2.0);
    Rect::from_center_size(available.center(), vec2(width, width / 2.0))
}

#[cfg(test)]
mod tests {
    use eframe::egui::Pos2;

    use super::*;

    fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect::from_min_size(Pos2::new(x, y), vec2(w, h))
    }

    #[test]
    fn a_wide_area_gets_side_bars() {
        let fitted = panorama_rect(rect(10.0, 20.0, 1000.0, 300.0));

        assert_eq!(fitted, rect(10.0 + 200.0, 20.0, 600.0, 300.0));
    }

    #[test]
    fn a_tall_area_gets_bars_above_and_below() {
        let fitted = panorama_rect(rect(0.0, 0.0, 400.0, 600.0));

        assert_eq!(fitted, rect(0.0, 200.0, 400.0, 200.0));
    }

    #[test]
    fn an_exactly_two_to_one_area_is_used_whole() {
        let area = rect(5.0, 5.0, 800.0, 400.0);

        assert_eq!(panorama_rect(area), area);
    }
}
