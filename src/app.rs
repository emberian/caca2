use std::sync::Arc;
use winit::{
    dpi::PhysicalSize,
    event::{ElementState, KeyEvent, MouseButton, WindowEvent},
    keyboard::{KeyCode, PhysicalKey},
    window::Window,
};

use crate::gpu::GpuContext;
use crate::simulation::{Simulation, SimulationConfig, patterns};
use crate::ui::{self, UiState, Tool};

pub struct App {
    gpu: GpuContext,
    egui_renderer: egui_wgpu::Renderer,
    egui_state: egui_winit::State,
    egui_ctx: egui::Context,
    ui_state: UiState,
    simulation: Simulation,
    window: Arc<Window>,
    frame_count: u64,
    mouse_pos: Option<(f32, f32)>,
    mouse_down: bool,
}

pub struct EventResponse {
    pub consumed: bool,
}

impl App {
    pub async fn new(window: Arc<Window>) -> Self {
        let gpu = GpuContext::new(window.clone()).await;
        
        let config = SimulationConfig::default();
        let simulation = Simulation::new(config);
        
        let egui_ctx = egui::Context::default();
        let egui_state = egui_winit::State::new(
            egui_ctx.clone(),
            egui::ViewportId::ROOT,
            &window,
            Some(window.scale_factor() as f32),
            None,
            None,
        );
        
        let egui_renderer = egui_wgpu::Renderer::new(
            &gpu.device,
            gpu.config.format,
            None,
            1,
            false,
        );
        
        let mut ui_state = UiState::default();
        ui_state.sync_from_rule(&simulation.config.rule);
        
        Self {
            gpu,
            egui_renderer,
            egui_state,
            egui_ctx,
            ui_state,
            simulation,
            window,
            frame_count: 0,
            mouse_pos: None,
            mouse_down: false,
        }
    }
    
    pub fn handle_event(&mut self, event: &WindowEvent) -> EventResponse {
        let response = self.egui_state.on_window_event(&self.window, event);
        
        if !response.consumed {
            match event {
                WindowEvent::CursorMoved { position, .. } => {
                    self.mouse_pos = Some((position.x as f32, position.y as f32));
                    if self.mouse_down {
                        self.handle_mouse_drag();
                    }
                }
                WindowEvent::MouseInput { state, button, .. } => {
                    if *button == MouseButton::Left {
                        self.mouse_down = *state == ElementState::Pressed;
                        if self.mouse_down {
                            self.handle_mouse_click();
                        }
                    }
                }
                _ => {}
            }
        }
        
        EventResponse { consumed: response.consumed }
    }
    
    pub fn handle_keyboard(&mut self, event: &KeyEvent) {
        if !event.state.is_pressed() { return; }
        
        match event.physical_key {
            PhysicalKey::Code(KeyCode::Space) => {
                self.ui_state.paused = !self.ui_state.paused;
            }
            PhysicalKey::Code(KeyCode::KeyR) => {
                self.simulation.randomize(None);
            }
            PhysicalKey::Code(KeyCode::KeyC) => {
                self.simulation.clear();
            }
            PhysicalKey::Code(KeyCode::Period) | PhysicalKey::Code(KeyCode::KeyN) => {
                if self.ui_state.paused {
                    self.simulation.step();
                }
            }
            PhysicalKey::Code(KeyCode::Comma) | PhysicalKey::Code(KeyCode::KeyB) => {
                if self.ui_state.paused {
                    self.simulation.step_back();
                }
            }
            PhysicalKey::Code(KeyCode::Digit1) => self.ui_state.current_tool = Tool::Pan,
            PhysicalKey::Code(KeyCode::Digit2) => self.ui_state.current_tool = Tool::Draw,
            PhysicalKey::Code(KeyCode::Digit3) => self.ui_state.current_tool = Tool::Erase,
            PhysicalKey::Code(KeyCode::Digit4) => self.ui_state.current_tool = Tool::Place,
            _ => {}
        }
    }
    
    fn handle_mouse_click(&mut self) {
        self.apply_tool_at_mouse();
    }
    
    fn handle_mouse_drag(&mut self) {
        self.apply_tool_at_mouse();
    }
    
    fn apply_tool_at_mouse(&mut self) {
        if let Some((mx, my)) = self.mouse_pos {
            let (cell_x, cell_y) = self.screen_to_cell(mx, my);
            
            match self.ui_state.current_tool {
                Tool::Draw => {
                    self.simulation.set_cell(cell_x, cell_y, 1);
                }
                Tool::Erase => {
                    self.simulation.set_cell(cell_x, cell_y, 0);
                }
                Tool::Place => {
                    let all_patterns = patterns::all();
                    if self.ui_state.selected_pattern < all_patterns.len() {
                        let pattern = &all_patterns[self.ui_state.selected_pattern];
                        self.simulation.place_pattern(pattern, cell_x, cell_y);
                    }
                }
                Tool::Pan => {}
            }
        }
    }
    
    fn screen_to_cell(&self, screen_x: f32, screen_y: f32) -> (u32, u32) {
        let panel_width = 280.0;
        let top_bar_height = 30.0;
        
        let canvas_x = (screen_x - panel_width).max(0.0);
        let canvas_y = (screen_y - top_bar_height).max(0.0);
        
        let canvas_width = (self.gpu.config.width as f32 - panel_width).max(1.0);
        let canvas_height = (self.gpu.config.height as f32 - top_bar_height).max(1.0);
        
        let norm_x = canvas_x / canvas_width;
        let norm_y = canvas_y / canvas_height;
        
        let cell_x = (norm_x * self.simulation.width() as f32) as u32;
        let cell_y = (norm_y * self.simulation.height() as f32) as u32;
        
        (
            cell_x.min(self.simulation.width().saturating_sub(1)),
            cell_y.min(self.simulation.height().saturating_sub(1)),
        )
    }
    
    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        self.gpu.resize(size);
    }
    
    pub fn update(&mut self) {
        if self.ui_state.paused {
            return;
        }
        
        let speed_frames = 61 - self.ui_state.speed.min(60) as u64;
        if self.frame_count % speed_frames.max(1) == 0 {
            self.simulation.step();
        }
        
        self.frame_count += 1;
    }
    
    pub fn render(&mut self) {
        let output = match self.gpu.surface.get_current_texture() {
            Ok(t) => t,
            Err(_) => {
                self.gpu.surface.configure(&self.gpu.device, &self.gpu.config);
                return;
            }
        };
        
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        
        let raw_input = self.egui_state.take_egui_input(&self.window);
        
        let full_output = self.egui_ctx.run(raw_input, |ctx| {
            ui::draw_ui(ctx, &mut self.ui_state, &mut self.simulation);
        });
        
        self.egui_state.handle_platform_output(&self.window, full_output.platform_output);
        
        let paint_jobs = self.egui_ctx.tessellate(full_output.shapes, full_output.pixels_per_point);
        
        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [self.gpu.config.width, self.gpu.config.height],
            pixels_per_point: full_output.pixels_per_point,
        };
        
        for (id, image_delta) in &full_output.textures_delta.set {
            self.egui_renderer.update_texture(&self.gpu.device, &self.gpu.queue, *id, image_delta);
        }
        
        let mut encoder = self.gpu.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });
        
        self.egui_renderer.update_buffers(
            &self.gpu.device,
            &self.gpu.queue,
            &mut encoder,
            &paint_jobs,
            &screen_descriptor,
        );
        
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Main Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.02,
                            g: 0.02,
                            b: 0.05,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            
            self.render_grid(&mut render_pass);
        }
        
        {
            let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Egui Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            
            let mut render_pass = render_pass.forget_lifetime();
            self.egui_renderer.render(&mut render_pass, &paint_jobs, &screen_descriptor);
        }
        
        self.gpu.queue.submit(std::iter::once(encoder.finish()));
        output.present();
        
        for id in &full_output.textures_delta.free {
            self.egui_renderer.free_texture(id);
        }
    }
    
    fn render_grid(&self, _render_pass: &mut wgpu::RenderPass) {
        // TODO: Implement proper GPU-based grid rendering
        // For now this is a placeholder - will add proper shader-based rendering
    }
}
