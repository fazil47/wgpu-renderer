use std::sync::Arc;

use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy},
    window::{Window, WindowId},
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::spawn_local;
#[cfg(target_arch = "wasm32")]
use winit::platform::web::WindowExtWebSys;

#[cfg(not(target_arch = "wasm32"))]
use crate::utils::load_icon;

use crate::engine::Engine;

pub struct StateInitializationEvent(Engine);

pub enum State {
    Uninitialized,
    Initializing,
    Initialized(Engine),
}

pub struct Application {
    application_state: State,
    event_loop_proxy: EventLoopProxy<StateInitializationEvent>,
}

impl Application {
    pub fn new(event_loop: &EventLoop<StateInitializationEvent>) -> Application {
        Application {
            application_state: State::Uninitialized,
            event_loop_proxy: event_loop.create_proxy(),
        }
    }
}

impl ApplicationHandler<StateInitializationEvent> for Application {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        match self.application_state {
            State::Initializing | State::Initialized(_) => return,
            State::Uninitialized => {
                self.application_state = State::Initializing;
            } // Continue
        }

        let window_attributes = Window::default_attributes().with_title("TODO: Change this");
        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

        #[cfg(target_arch = "wasm32")]
        {
            web_sys::window()
                .and_then(|win| win.document())
                .and_then(|doc| {
                    let dst = doc.get_element_by_id("engine-root")?;
                    let canvas = window.canvas()?;
                    canvas
                        .set_attribute("tabindex", "0")
                        .expect("failed to set tabindex");
                    dst.append_child(&canvas).ok()?;
                    canvas.focus().expect("Unable to focus on canvas");
                    Some(())
                })
                .expect("Couldn't append canvas to document body.");
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let icon = load_icon(std::path::Path::new("assets/icon.png"));
            window.set_window_icon(Some(icon));
        }

        let engine_future = Engine::new(window);

        #[cfg(target_arch = "wasm32")]
        {
            let event_loop_proxy = self.event_loop_proxy.clone();
            spawn_local(async move {
                let engine = engine_future.await;

                event_loop_proxy
                    .send_event(StateInitializationEvent(engine))
                    .unwrap_or_else(|_| {
                        panic!("Failed to send initialization event");
                    });
            });
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let engine = pollster::block_on(engine_future);

            self.event_loop_proxy
                .send_event(StateInitializationEvent(engine))
                .unwrap_or_else(|_| {
                    panic!("Failed to send initialization event");
                });
        }
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: StateInitializationEvent) {
        log::info!("Received initialization event");

        let engine = event.0;
        engine.window.request_redraw();
        self.application_state = State::Initialized(engine);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        let State::Initialized(ref mut engine) = self.application_state else {
            return;
        };

        let egui_event_response = engine
            .renderer
            .egui
            .state
            .on_window_event(&engine.window, &event);

        if egui_event_response.repaint {
            engine.window.request_redraw();
        }

        engine.input(&event);
        if engine.camera_controller.is_cursor_locked() {
            engine.update();
        }

        if egui_event_response.consumed {
            return;
        }

        match event {
            WindowEvent::Resized(new_size) => engine.resize(new_size),

            WindowEvent::RedrawRequested => engine.render().unwrap(),

            WindowEvent::CloseRequested => event_loop.exit(),

            _ => {}
        };
    }
}
