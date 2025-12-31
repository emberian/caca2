mod app;
mod gpu;
pub mod neighborhood;
pub mod rule;
pub mod simulation;
mod ui;

use winit::{
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    window::Window,
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
    
    run().await;
}

#[cfg(not(target_arch = "wasm32"))]
pub fn main() {
    env_logger::init();
    pollster::block_on(run());
}

async fn run() {
    let event_loop = EventLoop::new().expect("Failed to create event loop");
    
    #[allow(unused_mut)]
    let mut window_attrs = Window::default_attributes()
        .with_title("CACA - Cellular Automata Canvas")
        .with_inner_size(PhysicalSize::new(1280, 720));
    
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::JsCast;
        use winit::platform::web::WindowAttributesExtWebSys;
        let canvas = web_sys::window()
            .and_then(|win| win.document())
            .and_then(|doc| doc.get_element_by_id("canvas"))
            .and_then(|el| el.dyn_into::<web_sys::HtmlCanvasElement>().ok())
            .expect("Failed to get canvas element");
        
        window_attrs = window_attrs.with_canvas(Some(canvas));
    }
    
    let window = event_loop.create_window(window_attrs).expect("Failed to create window");
    
    #[cfg(target_arch = "wasm32")]
    {
        use winit::dpi::PhysicalSize;
        let _ = window.request_inner_size(PhysicalSize::new(
            web_sys::window().unwrap().inner_width().unwrap().as_f64().unwrap() as u32,
            web_sys::window().unwrap().inner_height().unwrap().as_f64().unwrap() as u32,
        ));
    }
    
    let window = std::sync::Arc::new(window);
    let mut app = App::new(window.clone()).await;
    
    event_loop.run(move |event, target| {
        match event {
            Event::WindowEvent { event, .. } => {
                let response = app.handle_event(&event);
                
                if !response.consumed {
                    match event {
                        WindowEvent::CloseRequested => target.exit(),
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
            }
            Event::AboutToWait => {
                window.request_redraw();
            }
            _ => {}
        }
    }).expect("Event loop error");
}
