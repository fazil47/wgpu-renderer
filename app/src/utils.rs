#[cfg(not(target_arch = "wasm32"))]
pub fn load_icon(path: &std::path::Path) -> winit::window::Icon {
    let (icon_rgba, icon_width, icon_height) = {
        let image = image::open(path)
            .expect("Failed to open icon path")
            .into_rgba8();
        let (width, height) = image.dimensions();
        let rgba = image.into_raw();
        (rgba, width, height)
    };

    winit::window::Icon::from_rgba(icon_rgba, icon_width, icon_height).expect("Failed to open icon")
}
