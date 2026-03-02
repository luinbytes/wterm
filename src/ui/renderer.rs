//! GPU-accelerated renderer using wgpu.
//!
//! This module provides the main rendering infrastructure for the terminal emulator,
//! including text rendering via the text module.

#![allow(dead_code)]

use thiserror::Error;
use wgpu::{Device, PresentMode, Queue, Surface, SurfaceConfiguration, TextureViewDescriptor};
use winit::window::Window;

use super::text::{TextError, TextRenderer};
use crate::terminal::grid::TerminalGrid;

#[derive(Error, Debug)]
pub enum RendererError {
    #[error("Failed to create wgpu surface: {0}")]
    SurfaceCreation(String),

    #[error("Failed to request adapter: {0}")]
    AdapterRequest(String),

    #[error("Failed to request device: {0}")]
    DeviceRequest(String),

    #[error("Failed to configure surface")]
    SurfaceConfiguration,

    #[error("Failed to get current texture: {0}")]
    TextureAcquisition(String),

    #[error("Render error: {0}")]
    Render(String),

    #[error("Text rendering error: {0}")]
    Text(#[from] TextError),
}

/// Default font size in pixels
const DEFAULT_FONT_SIZE: f32 = 16.0;

/// GPU-accelerated renderer using wgpu
pub struct Renderer<'window> {
    device: Device,
    queue: Queue,
    surface: Surface<'window>,
    config: SurfaceConfiguration,
    render_pipeline: wgpu::RenderPipeline,
    /// Text renderer for terminal content
    text_renderer: TextRenderer,
    /// Text bind group (recreated each frame if needed)
    text_bind_group: Option<wgpu::BindGroup>,
}

impl<'window> Renderer<'window> {
    /// Create a new renderer instance
    pub async fn new(window: &'window Window) -> Result<Self, RendererError> {
        // Create instance
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        // Create surface
        let surface = instance
            .create_surface(window)
            .map_err(|e| RendererError::SurfaceCreation(e.to_string()))?;

        // Request adapter
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or_else(|| {
                RendererError::AdapterRequest("No suitable adapter found".to_string())
            })?;

        // Get surface capabilities
        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let size = window.inner_size();

        // Request device and queue
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    label: None,
                    memory_hints: Default::default(),
                },
                None,
            )
            .await
            .map_err(|e| RendererError::DeviceRequest(e.to_string()))?;

        // Configure surface
        let config = SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: PresentMode::Fifo, // Use Fifo (vsync) for maximum cross-platform compatibility
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&device, &config);

        // Create a basic render pipeline (placeholder for now)
        let render_pipeline = Self::create_render_pipeline(&device, config.format);

        // Create text renderer
        let mut text_renderer =
            TextRenderer::new(&device, DEFAULT_FONT_SIZE, (config.width, config.height))?;
        text_renderer.init_pipeline(&device, config.format);

        // Create initial bind group
        let text_bind_group = text_renderer.create_bind_group(&device);

        Ok(Self {
            device,
            queue,
            surface,
            config,
            render_pipeline,
            text_renderer,
            text_bind_group,
        })
    }

    /// Create a basic render pipeline
    fn create_render_pipeline(
        device: &Device,
        format: wgpu::TextureFormat,
    ) -> wgpu::RenderPipeline {
        // Create empty shader for now (we'll add actual shaders later)
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Basic Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/basic.wgsl").into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        })
    }

    /// Resize the renderer surface
    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
            self.text_renderer.resize(width, height);
        }
    }

    /// Render a frame with the terminal grid content
    pub fn render_grid(&mut self, grid: &TerminalGrid) -> Result<(), RendererError> {
        // Clear any previous frame's text
        self.text_renderer.clear();

        // Calculate cell dimensions
        let font_size = self.text_renderer.font_size();
        let cell_width = font_size * 0.6; // Approximate monospace character width
        let cell_height = font_size;

        // Render all visible cells
        let rows = grid.rows();
        let cols = grid.cols();

        for row in 0..rows {
            for col in 0..cols {
                if let Some(cell) = grid.get_cell(row, col) {
                    if cell.char != ' ' {
                        let x = col as f32 * cell_width;
                        let y = row as f32 * cell_height;

                        self.text_renderer.queue_char(
                            cell.char,
                            x,
                            y,
                            cell.fg_color,
                            cell.bg_color,
                            cell.attributes.bold,
                            cell.attributes.italic,
                            cell.attributes.underline,
                            cell.attributes.blink,
                        )?;
                    }
                }
            }
        }

        // Prepare text renderer (upload glyph atlas and vertex data)
        self.text_renderer.prepare(&self.device, &self.queue);

        // Update bind group if needed
        if self.text_bind_group.is_none() {
            self.text_bind_group = self.text_renderer.create_bind_group(&self.device);
        }

        // Render the frame
        self.render()
    }

    /// Render a frame (basic clear only)
    pub fn render(&mut self) -> Result<(), RendererError> {
        let output = self
            .surface
            .get_current_texture()
            .map_err(|e| RendererError::TextureAcquisition(e.to_string()))?;

        let view = output
            .texture
            .create_view(&TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.01,
                            g: 0.01,
                            b: 0.01,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            // Render text if we have vertices and bind group
            if let Some(ref bind_group) = self.text_bind_group {
                if self.text_renderer.vertex_count() > 0 {
                    self.text_renderer.render(&mut render_pass, bind_group);
                }
            }
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }

    /// Get reference to device
    pub fn device(&self) -> &Device {
        &self.device
    }

    /// Get reference to queue
    pub fn queue(&self) -> &Queue {
        &self.queue
    }

    /// Get current surface configuration
    pub fn config(&self) -> &SurfaceConfiguration {
        &self.config
    }

    /// Get the font size
    pub fn font_size(&self) -> f32 {
        self.text_renderer.font_size()
    }

    /// Calculate terminal dimensions based on current window size
    pub fn terminal_dimensions(&self) -> (usize, usize) {
        let font_size = self.text_renderer.font_size();
        let cell_width = font_size * 0.6;
        let cell_height = font_size;

        let cols = (self.config.width as f32 / cell_width).floor() as usize;
        let rows = (self.config.height as f32 / cell_height).floor() as usize;

        (cols.max(1), rows.max(1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = RendererError::SurfaceCreation("test".to_string());
        assert!(err.to_string().contains("test"));
    }

    #[test]
    fn test_default_font_size() {
        assert_eq!(DEFAULT_FONT_SIZE, 16.0);
    }

    #[test]
    fn test_renderer_error_adapter_request() {
        let err = RendererError::AdapterRequest("No suitable GPU found".to_string());
        let msg = err.to_string();
        assert!(msg.contains("Failed to request adapter"));
        assert!(msg.contains("No suitable GPU found"));
    }

    #[test]
    fn test_renderer_error_device_request() {
        let err = RendererError::DeviceRequest("Device limit exceeded".to_string());
        let msg = err.to_string();
        assert!(msg.contains("Failed to request device"));
        assert!(msg.contains("Device limit exceeded"));
    }

    #[test]
    fn test_renderer_error_surface_configuration() {
        let err = RendererError::SurfaceConfiguration;
        let msg = err.to_string();
        assert!(msg.contains("Failed to configure surface"));
    }

    #[test]
    fn test_renderer_error_texture_acquisition() {
        let err = RendererError::TextureAcquisition("Texture lost".to_string());
        let msg = err.to_string();
        assert!(msg.contains("Failed to get current texture"));
        assert!(msg.contains("Texture lost"));
    }

    #[test]
    fn test_renderer_error_render() {
        let err = RendererError::Render("Pipeline error".to_string());
        let msg = err.to_string();
        assert!(msg.contains("Render error"));
        assert!(msg.contains("Pipeline error"));
    }

    #[test]
    fn test_renderer_error_text_from_text_error() {
        let text_err = TextError::FontLoad("Font not found".to_string());
        let renderer_err: RendererError = text_err.into();
        let msg = renderer_err.to_string();
        assert!(msg.contains("Text rendering error"));
        assert!(msg.contains("Font not found"));
    }

    #[test]
    fn test_renderer_error_debug() {
        let err = RendererError::SurfaceCreation("debug test".to_string());
        // Debug trait should work without panic
        let _debug_str = format!("{:?}", err);
    }

    #[test]
    fn test_renderer_error_surface_creation_empty_message() {
        let err = RendererError::SurfaceCreation(String::new());
        let msg = err.to_string();
        assert!(msg.contains("Failed to create wgpu surface"));
    }

    #[test]
    fn test_renderer_error_adapter_request_empty_message() {
        let err = RendererError::AdapterRequest(String::new());
        let msg = err.to_string();
        assert!(msg.contains("Failed to request adapter"));
    }

    #[test]
    fn test_renderer_error_render_with_multiline() {
        let err = RendererError::Render("Line 1\nLine 2\nLine 3".to_string());
        let msg = err.to_string();
        assert!(msg.contains("Render error"));
        assert!(msg.contains("Line 1"));
        assert!(msg.contains("Line 3"));
    }
}
