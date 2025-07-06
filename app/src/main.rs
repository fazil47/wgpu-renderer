use app::core::{Application, StateInitializationEvent};

#[cfg(target_arch = "wasm32")]
use winit::platform::web::EventLoopExtWebSys;

use winit::event_loop::EventLoop;

fn main() {
    run();
}

#[cfg(target_arch = "wasm32")]
mod wasm {
    use wasm_bindgen::prelude::*;
    #[wasm_bindgen(start)]
    pub fn run() {
        crate::run();
    }
}

/// # Panics
pub fn run() {
    #[cfg(target_arch = "wasm32")]
    {
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));
        console_log::init_with_level(log::Level::Warn).expect("Couldn't initialize logger");
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        env_logger::builder()
            .filter(None, log::LevelFilter::Warn)
            .filter(Some("wgpu_hal::vulkan"), log::LevelFilter::Error)
            .init();
    }

    let event_loop = EventLoop::<StateInitializationEvent>::with_user_event()
        .build()
        .unwrap();

    #[cfg(target_arch = "wasm32")]
    {
        let application: Application = Application::new(&event_loop);
        event_loop.spawn_app(application);
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let mut application: Application = Application::new(&event_loop);
        event_loop.run_app(&mut application).unwrap();
    }
}
