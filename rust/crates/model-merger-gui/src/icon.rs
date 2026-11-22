//! Shared artwork for window branding and the in-app icon.

use eframe::egui;
use std::sync::{Arc, OnceLock};

pub(crate) fn data() -> Arc<egui::IconData> {
    static ICON: OnceLock<Arc<egui::IconData>> = OnceLock::new();
    Arc::clone(ICON.get_or_init(|| {
        let png = include_bytes!(concat!(env!("OUT_DIR"), "/app-icon.png"));
        let icon = eframe::icon_data::from_png_bytes(png)
            .expect("the embedded application icon must be a valid PNG");
        Arc::new(icon)
    }))
}

pub(crate) fn texture(context: &egui::Context) -> egui::TextureHandle {
    let id = egui::Id::new("cast-assembly-icon");
    if let Some(texture) = context.data(|state| state.get_temp::<egui::TextureHandle>(id)) {
        return texture;
    }
    let icon = data();
    let image = egui::ColorImage::from_rgba_unmultiplied(
        [icon.width as usize, icon.height as usize],
        &icon.rgba,
    );
    let texture = context.load_texture("cast-assembly-icon", image, egui::TextureOptions::LINEAR);
    context.data_mut(|state| state.insert_temp(id, texture.clone()));
    texture
}

#[cfg(test)]
mod tests {
    use super::{data, texture};
    use eframe::egui;
    use std::sync::Arc;

    #[test]
    fn window_and_in_app_icons_share_the_artwork_and_texture() {
        let icon = data();
        assert_eq!((icon.width, icon.height), (256, 256));
        assert!(Arc::ptr_eq(&icon, &data()), "decoded icon should be reused");
        let context = egui::Context::default();
        let title_icon = texture(&context);
        let about_icon = texture(&context);
        assert_eq!(title_icon.id(), about_icon.id());
        assert_eq!(title_icon.size(), [256, 256]);
    }
}
