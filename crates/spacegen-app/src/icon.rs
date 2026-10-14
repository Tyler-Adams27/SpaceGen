//! The app icon: the window and taskbar picture, built from `assets/icon.png`
//! (a rasterised `assets/logo.svg`).

use eframe::egui::IconData;

/// The icon as RGBA pixels for the window.
pub fn app_icon() -> IconData {
    eframe::icon_data::from_png_bytes(include_bytes!("../../../assets/icon.png"))
        .expect("assets/icon.png is a valid PNG")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn alpha_at(icon: &IconData, x: u32, y: u32) -> u8 {
        icon.rgba[((y * icon.width + x) * 4 + 3) as usize]
    }

    #[test]
    fn icon_is_a_full_size_rgba_picture() {
        let icon = app_icon();
        assert_eq!((icon.width, icon.height), (256, 256));
        assert_eq!(icon.rgba.len(), 256 * 256 * 4);
    }

    #[test]
    fn icon_is_a_rounded_tile_with_transparent_corners_and_an_opaque_middle() {
        let icon = app_icon();
        assert_eq!(alpha_at(&icon, 0, 0), 0, "corner should be transparent");
        assert_eq!(alpha_at(&icon, 255, 255), 0, "corner should be transparent");
        assert_eq!(alpha_at(&icon, 128, 128), 255, "middle should be opaque");
    }
}
