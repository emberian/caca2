mod app;
mod gpu;
pub mod neighborhood;
pub mod rule;
pub mod simulation;
mod ui;

use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
    dpi::PhysicalSize,
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

pub use app::App;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub async fn start() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    console_log::init_with_level(log::Level::Info).expect("Failed to init logger");

    run_web().await;
}

#[cfg(target_arch = "wasm32")]
async fn run_web() {
    use wasm_bindgen::JsCast;
    use winit::platform::web::WindowAttributesExtWebSys;

    let event_loop = EventLoop::new().expect("Failed to create event loop");

    let canvas = web_sys::window()
        .and_then(|win| win.document())
        .and_then(|doc| doc.get_element_by_id("canvas"))
        .and_then(|el| el.dyn_into::<web_sys::HtmlCanvasElement>().ok())
        .expect("Failed to get canvas element");

    let window_attrs = Window::default_attributes()
        .with_title("CACA - Cellular Automata Canvas")
        .with_inner_size(PhysicalSize::new(1280, 720))
        .with_canvas(Some(canvas));

    #[allow(deprecated)]
    let window = event_loop
        .create_window(window_attrs)
        .expect("Failed to create window");

    let web_window = web_sys::window().unwrap();
    let dpr = web_window.device_pixel_ratio();
    let width = web_window.inner_width().unwrap().as_f64().unwrap();
    let height = web_window.inner_height().unwrap().as_f64().unwrap();
    let _ = window.request_inner_size(PhysicalSize::new(
        (width * dpr) as u32,
        (height * dpr) as u32,
    ));

    let window = Arc::new(window);
    let app = App::new(window.clone()).await;

    let window_clone = window.clone();
    let resize_closure = wasm_bindgen::closure::Closure::<dyn Fn()>::new(move || {
        let web_window = web_sys::window().unwrap();
        let dpr = web_window.device_pixel_ratio();
        let width = web_window.inner_width().unwrap().as_f64().unwrap();
        let height = web_window.inner_height().unwrap().as_f64().unwrap();
        let _ = window_clone.request_inner_size(PhysicalSize::new(
            (width * dpr) as u32,
            (height * dpr) as u32,
        ));
    });

    web_window
        .add_event_listener_with_callback("resize", resize_closure.as_ref().unchecked_ref())
        .unwrap();
    web_window
        .add_event_listener_with_callback("orientationchange", resize_closure.as_ref().unchecked_ref())
        .unwrap();
    resize_closure.forget();

    let mut handler = WebAppHandler { window, app };
    event_loop.run_app(&mut handler).expect("Event loop error");
}

#[cfg(target_arch = "wasm32")]
struct WebAppHandler {
    window: Arc<Window>,
    app: App,
}

#[cfg(target_arch = "wasm32")]
impl ApplicationHandler for WebAppHandler {
    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {}

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let response = self.app.handle_event(&event);

        if !response.consumed {
            match event {
                WindowEvent::CloseRequested => event_loop.exit(),
                WindowEvent::Resized(size) => {
                    self.app.resize(size);
                }
                WindowEvent::RedrawRequested => {
                    self.app.update();
                    self.app.render();
                }
                WindowEvent::KeyboardInput { event, .. } => {
                    self.app.handle_keyboard(&event);
                }
                _ => {}
            }
        }

        self.window.request_redraw();
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        self.window.request_redraw();
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn main() {
    env_logger::init();
    run_native();
}

#[cfg(not(target_arch = "wasm32"))]
struct NativeAppHandler {
    window: Option<Arc<Window>>,
    app: Option<App>,
}

#[cfg(not(target_arch = "wasm32"))]
impl NativeAppHandler {
    fn new() -> Self {
        Self {
            window: None,
            app: None,
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl ApplicationHandler for NativeAppHandler {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let window_attrs = Window::default_attributes()
            .with_title("CACA - Cellular Automata Canvas")
            .with_inner_size(PhysicalSize::new(1280, 720));

        let window = event_loop
            .create_window(window_attrs)
            .expect("Failed to create window");

        let window = Arc::new(window);
        let app = pollster::block_on(App::new(window.clone()));

        self.window = Some(window);
        self.app = Some(app);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let Some(app) = self.app.as_mut() else {
            return;
        };
        let Some(window) = self.window.as_ref() else {
            return;
        };

        let response = app.handle_event(&event);

        if !response.consumed {
            match event {
                WindowEvent::CloseRequested => event_loop.exit(),
                WindowEvent::Resized(size) => {
                    app.resize(size);
                }
                WindowEvent::RedrawRequested => {
                    app.update();
                    app.render();
                }
                WindowEvent::KeyboardInput { event, .. } => {
                    app.handle_keyboard(&event);
                }
                _ => {}
            }
        }

        window.request_redraw();
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn run_native() {
    let event_loop = EventLoop::new().expect("Failed to create event loop");
    let mut handler = NativeAppHandler::new();
    event_loop.run_app(&mut handler).expect("Event loop error");
}
