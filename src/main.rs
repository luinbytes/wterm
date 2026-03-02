//! Warp FOSS - A free terminal with AI integration
//!
//! Main entry point with event loop that ties together:
//! - winit window management
//! - wgpu rendering
//! - PTY session for shell I/O
//! - Terminal parsing and grid state
//! - Layout management for split panes

mod ai;
mod config;
mod plugin;
mod search;
mod terminal;
mod ui;

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::Result;
use terminal::grid::TerminalGrid;
use terminal::parser::TerminalParser;
use terminal::pty::{PtyConfig, PtySession};
use ui::ai_command_palette::AICommandPalette;
use ui::input::InputHandler;
use search::SearchState;
use ui::layout::{LayoutTree, Pane, Rect, SplitDirection};
use ui::selection::{extract_selected_text, Clipboard, SelectionState};
use ui::status_bar::StatusBar;
use winit::{
    application::ApplicationHandler,
    dpi::{PhysicalPosition, PhysicalSize},
    event::{DeviceId, ElementState, MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{Key, ModifiersState, NamedKey},
    window::{Window, WindowId},
};

/// Configuration for the terminal application
struct AppConfig {
    /// Initial terminal columns
    cols: u16,
    /// Initial terminal rows  
    rows: u16,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            cols: 120,
            rows: 40,
        }
    }
}

/// Main application state
struct TerminalApp {
    /// The winit window
    window: Option<Arc<Window>>,
    /// GPU renderer - stored as raw parts to avoid lifetime issues
    renderer: Option<RendererHolder>,
    /// Layout tree managing all panes
    layout: Option<LayoutTree>,
    /// Input handler for keyboard events
    input_handler: InputHandler,
    /// Selection state for mouse selection
    selection_state: SelectionState,
    /// Clipboard manager
    clipboard: Clipboard,
    /// Whether the app is running
    running: bool,
    /// Last frame time for FPS limiting
    last_frame: Instant,
    /// Target frame duration (60 FPS)
    frame_duration: Duration,
    /// Cell dimensions in pixels
    cell_width: u32,
    cell_height: u32,
    /// Current cursor position in pixels
    cursor_position: Option<PhysicalPosition<f64>>,
    /// Current modifier state
    modifiers: ModifiersState,
    /// Search state
    search_state: SearchState,
    /// Search mode input buffer
    search_input: String,
    /// AI command palette
    ai_palette: AICommandPalette,
    /// Status bar
    status_bar: StatusBar,
}

/// Type-erased renderer holder to work around lifetime issues
struct RendererHolder {
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    text_renderer: ui::text::TextRenderer,
    text_bind_group: Option<wgpu::BindGroup>,
}

impl RendererHolder {
    async fn new(window: Arc<Window>) -> Result<Self, ui::renderer::RendererError> {
        use ui::renderer::RendererError;

        // Create instance - prefer DX12 on Windows for better cross-compile compatibility
        #[cfg(target_os = "windows")]
        let backends = wgpu::Backends::DX12;
        #[cfg(not(target_os = "windows"))]
        let backends = wgpu::Backends::all();

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends,
            ..Default::default()
        });

        // Create surface - we need 'static lifetime, so we leak the Arc
        // This is safe because the window lives for the duration of the application
        let window_static: &'static Window = Box::leak(Box::new(window));

        let surface = instance
            .create_surface(window_static)
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

        let size = window_static.inner_size();

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
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo, // Use Fifo (vsync) for maximum cross-platform compatibility
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&device, &config);

        // Create text renderer
        let window_size = (size.width, size.height);
        let mut text_renderer = ui::text::TextRenderer::new(&device, 16.0, window_size)?;
        
        // Initialize the render pipeline (creates bind_group_layout)
        text_renderer.init_pipeline(&device, surface_format);

        // Create bind group for text
        let text_bind_group = text_renderer.create_bind_group(&device);

        Ok(Self {
            device,
            queue,
            surface,
            config,
            text_renderer,
            text_bind_group,
        })
    }

    fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
            // Update text renderer screen size for correct NDC calculations
            self.text_renderer.resize(width, height);
        }
    }

    fn render(&mut self) -> Result<(), ui::renderer::RendererError> {
        use ui::renderer::RendererError;

        tracing::debug!("RendererHolder::render() - getting texture");

        let output = self
            .surface
            .get_current_texture()
            .map_err(|e| {
                tracing::error!("Failed to get current texture: {}", e);
                RendererError::TextureAcquisition(e.to_string())
            })?;

        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

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
                            r: 0.0,  // Pure black background
                            g: 0.0,
                            b: 0.0,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            // Render text if we have bind group and vertices
            if let Some(ref bind_group) = self.text_bind_group {
                if self.text_renderer.vertex_count() > 0 {
                    self.text_renderer.render(&mut render_pass, bind_group);
                }
            }
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        tracing::debug!("RendererHolder::render() - submitted commands, presenting");
        output.present();
        tracing::debug!("RendererHolder::render() - presented successfully");

        Ok(())
    }

    fn render_layout(
        &mut self,
        layout: &LayoutTree,
        cell_width: u32,
        cell_height: u32,
        focused_pane_id: uuid::Uuid,
        search_state: &SearchState,
        search_input: &str,
        ai_palette: &AICommandPalette,
        status_bar: &StatusBar,
    ) -> Result<(), ui::renderer::RendererError> {
        // Clear previous frame's text
        self.text_renderer.clear();

        // Render all panes in the layout
        self.render_node(layout.root(), cell_width, cell_height, focused_pane_id, search_state, search_input)?;

        // Render AI palette overlay if visible
        if ai_palette.is_visible() {
            self.render_ai_palette(ai_palette, cell_width, cell_height)?;
        }

        // Render status bar if visible
        if status_bar.is_visible() {
            self.render_status_bar(status_bar, cell_width, cell_height)?;
        }

        // Prepare text renderer (upload glyph atlas and vertex data)
        self.text_renderer.prepare(&self.device, &self.queue);

        // Render to screen
        self.render()
    }

    fn render_node(
        &mut self,
        node: &ui::layout::LayoutNode,
        cell_width: u32,
        cell_height: u32,
        focused_pane_id: uuid::Uuid,
        search_state: &SearchState,
        search_input: &str,
    ) -> Result<(), ui::renderer::RendererError> {
        use ui::layout::LayoutNode;

        match node {
            LayoutNode::Pane(pane) => {
                self.render_pane(pane, cell_width, cell_height, pane.id == focused_pane_id, search_state, search_input)?;
            }
            LayoutNode::HorizontalSplit { children, .. } => {
                for child in children {
                    self.render_node(child, cell_width, cell_height, focused_pane_id, search_state, search_input)?;
                }
            }
            LayoutNode::VerticalSplit { children, .. } => {
                for child in children {
                    self.render_node(child, cell_width, cell_height, focused_pane_id, search_state, search_input)?;
                }
            }
        }

        Ok(())
    }

    fn render_pane(
        &mut self,
        pane: &Pane,
        cell_width: u32,
        cell_height: u32,
        is_focused: bool,
        search_state: &SearchState,
        search_input: &str,
    ) -> Result<(), ui::renderer::RendererError> {
        use terminal::parser::Color;

        let bounds = pane.bounds;
        let grid = &pane.grid;

        // Render terminal content
        let rows = grid.rows();
        let cols = grid.cols();

        // Warp-style: content starts with small padding from window edges
        let padding = 8.0; // 8px padding
        let content_offset_x = bounds.x as f32 + padding;
        let content_offset_y = bounds.y as f32 + padding;

        // Render search bar if search is active on focused pane
        if is_focused && search_state.active {
            self.render_search_bar(bounds, cell_width, cell_height, search_state, search_input)?;
        }

        for row in 0..rows {
            for col in 0..cols {
                if let Some(cell) = grid.get_cell(row, col) {
                    if cell.char != ' ' {
                        // Offset by pane bounds + border offset
                        let x = content_offset_x + (col as f32 * cell_width as f32);
                        let y = content_offset_y + (row as f32 * cell_height as f32);

                        // Highlight search matches
                        let (fg_color, bg_color) = if is_focused && search_state.active {
                            if search_state.is_current_match(row, col) {
                                // Current match: bright yellow background
                                (Color::Rgb(0, 0, 0), Color::Rgb(255, 255, 0))
                            } else if search_state.is_match(row, col) {
                                // Other matches: orange background
                                (Color::Rgb(0, 0, 0), Color::Rgb(255, 165, 0))
                            } else {
                                (cell.fg_color, cell.bg_color)
                            }
                        } else {
                            (cell.fg_color, cell.bg_color)
                        };

                        self.text_renderer.queue_char(
                            cell.char,
                            x,
                            y,
                            fg_color,
                            bg_color,
                            cell.attributes.bold,
                            cell.attributes.italic,
                            cell.attributes.underline,
                            cell.attributes.blink,
                        )?;
                    }
                }
            }
        }

        // Draw pane borders
        self.draw_pane_borders(bounds, cell_width, cell_height, is_focused)?;

        Ok(())
    }

    fn render_search_bar(
        &mut self,
        bounds: Rect,
        cell_width: u32,
        cell_height: u32,
        search_state: &SearchState,
        search_input: &str,
    ) -> Result<(), ui::renderer::RendererError> {
        use terminal::parser::Color;

        let x = bounds.x as f32 + cell_width as f32;
        let y = bounds.y as f32 + cell_height as f32;
        let bar_width = (bounds.width / cell_width) as usize - 2; // -2 for borders

        // Draw search bar background
        let search_bg = Color::Rgb(40, 40, 40);
        let search_fg = Color::Rgb(255, 255, 255);
        let search_hint = Color::Rgb(150, 150, 150);

        // Draw "Search:" label
        let label = "Search: ";
        for (i, ch) in label.chars().enumerate() {
            self.text_renderer.queue_char(
                ch,
                x + (i as f32 * cell_width as f32),
                y,
                search_hint,
                search_bg,
                false,
                false,
                false,
                false,
            )?;
        }

        // Draw search input
        let input_start = label.len();
        for (i, ch) in search_input.chars().enumerate() {
            if i + input_start >= bar_width {
                break;
            }
            self.text_renderer.queue_char(
                ch,
                x + ((input_start + i) as f32 * cell_width as f32),
                y,
                search_fg,
                search_bg,
                false,
                false,
                false,
                false,
            )?;
        }

        // Draw cursor
        let cursor_pos = input_start + search_input.len();
        if cursor_pos < bar_width {
            self.text_renderer.queue_char(
                '▏',
                x + (cursor_pos as f32 * cell_width as f32),
                y,
                search_fg,
                search_bg,
                false,
                false,
                false,
                false,
            )?;
        }

        // Draw match count
        if search_state.match_count() > 0 {
            let match_text = format!(
                " {} / {} ",
                search_state.current_match_number().unwrap_or(0),
                search_state.match_count()
            );
            let match_start = bar_width.saturating_sub(match_text.len());

            for (i, ch) in match_text.chars().enumerate() {
                if match_start + i >= bar_width {
                    break;
                }
                self.text_renderer.queue_char(
                    ch,
                    x + ((match_start + i) as f32 * cell_width as f32),
                    y,
                    search_hint,
                    search_bg,
                    false,
                    false,
                    false,
                    false,
                )?;
            }
        }

        Ok(())
    }

    fn render_ai_palette(
        &mut self,
        ai_palette: &AICommandPalette,
        cell_width: u32,
        cell_height: u32,
    ) -> Result<(), ui::renderer::RendererError> {
        use crate::ui::ai_command_palette::PaletteState;
        use terminal::parser::Color;

        if !ai_palette.is_visible() {
            return Ok(());
        }

        // Calculate palette dimensions and position (centered)
        let palette_width = 80usize; // characters
        let palette_height = 10usize; // lines

        let surface_width = self.config.width;
        let surface_height = self.config.height;

        let palette_x = ((surface_width / cell_width).saturating_sub(palette_width as u32) / 2) as f32 * cell_width as f32;
        let palette_y = ((surface_height / cell_height).saturating_sub(palette_height as u32) / 2) as f32 * cell_height as f32;

        // Colors
        let bg_color = Color::Rgb(30, 30, 40);
        let border_color = Color::Rgb(100, 149, 237); // Cornflower blue
        let text_color = Color::Rgb(255, 255, 255);
        let hint_color = Color::Rgb(150, 150, 150);
        let cursor_color = Color::Rgb(255, 215, 0); // Gold

        // Draw background and border
        for row in 0..palette_height {
            for col in 0..palette_width {
                let char_x = palette_x + (col as f32 * cell_width as f32);
                let char_y = palette_y + (row as f32 * cell_height as f32);

                let (ch, fg, bg) = if row == 0 || row == palette_height - 1 {
                    // Top or bottom border
                    if col == 0 || col == palette_width - 1 {
                        ('+', border_color, bg_color)
                    } else {
                        ('-', border_color, bg_color)
                    }
                } else if col == 0 || col == palette_width - 1 {
                    // Side borders
                    ('|', border_color, bg_color)
                } else {
                    // Interior
                    (' ', text_color, bg_color)
                };

                self.text_renderer.queue_char(
                    ch,
                    char_x,
                    char_y,
                    fg,
                    bg,
                    false,
                    false,
                    false,
                    false,
                )?;
            }
        }

        // Draw title
        let title = match ai_palette.state {
            PaletteState::Open => " AI Command Palette ",
            PaletteState::Processing => " AI Processing... ",
            PaletteState::ShowingResponse => " AI Response ",
            _ => " AI Command Palette ",
        };

        let title_x = palette_x + (2.0 * cell_width as f32);
        let title_y = palette_y + cell_height as f32;

        for (i, ch) in title.chars().enumerate() {
            if i + 2 >= palette_width - 2 {
                break;
            }
            self.text_renderer.queue_char(
                ch,
                title_x + (i as f32 * cell_width as f32),
                title_y,
                border_color,
                bg_color,
                true,
                false,
                false,
                false,
            )?;
        }

        // Draw input prompt or response based on state
        let content_y = palette_y + (3.0 * cell_height as f32);
        let content_x = palette_x + (2.0 * cell_width as f32);
        let max_content_width = palette_width - 4;

        match ai_palette.state {
            PaletteState::Open => {
                // Draw prompt
                let prompt_label = "> ";
                for (i, ch) in prompt_label.chars().enumerate() {
                    self.text_renderer.queue_char(
                        ch,
                        content_x + (i as f32 * cell_width as f32),
                        content_y,
                        hint_color,
                        bg_color,
                        false,
                        false,
                        false,
                        false,
                    )?;
                }

                // Draw input text
                let input_start = prompt_label.len();
                for (i, ch) in ai_palette.input.chars().enumerate() {
                    if i + input_start >= max_content_width {
                        break;
                    }
                    self.text_renderer.queue_char(
                        ch,
                        content_x + ((input_start + i) as f32 * cell_width as f32),
                        content_y,
                        text_color,
                        bg_color,
                        false,
                        false,
                        false,
                        false,
                    )?;
                }

                // Draw cursor
                let cursor_pos = input_start + ai_palette.cursor_pos;
                if cursor_pos < max_content_width {
                    self.text_renderer.queue_char(
                        '▏',
                        content_x + (cursor_pos as f32 * cell_width as f32),
                        content_y,
                        cursor_color,
                        bg_color,
                        false,
                        false,
                        false,
                        false,
                    )?;
                }

                // Draw hint
                let hint = "Press Enter to submit, Esc to close";
                let hint_y = palette_y + ((palette_height - 2) as f32 * cell_height as f32);
                for (i, ch) in hint.chars().enumerate() {
                    if i >= max_content_width {
                        break;
                    }
                    self.text_renderer.queue_char(
                        ch,
                        content_x + (i as f32 * cell_width as f32),
                        hint_y,
                        hint_color,
                        bg_color,
                        false,
                        false,
                        false,
                        false,
                    )?;
                }
            }
            PaletteState::Processing => {
                // Draw processing indicator
                let processing_text = "⠋ Contacting AI...";
                for (i, ch) in processing_text.chars().enumerate() {
                    if i >= max_content_width {
                        break;
                    }
                    self.text_renderer.queue_char(
                        ch,
                        content_x + (i as f32 * cell_width as f32),
                        content_y,
                        hint_color,
                        bg_color,
                        false,
                        false,
                        false,
                        false,
                    )?;
                }
            }
            PaletteState::ShowingResponse => {
                // Draw response
                let response = ai_palette.get_response();
                let response_lines: Vec<&str> = response.lines().collect();

                for (line_idx, line) in response_lines.iter().enumerate() {
                    if line_idx >= palette_height - 5 {
                        break; // Don't overflow the palette
                    }

                    let line_y = content_y + (line_idx as f32 * cell_height as f32);

                    for (i, ch) in line.chars().enumerate() {
                        if i >= max_content_width {
                            break;
                        }
                        self.text_renderer.queue_char(
                            ch,
                            content_x + (i as f32 * cell_width as f32),
                            line_y,
                            text_color,
                            bg_color,
                            false,
                            false,
                            false,
                            false,
                        )?;
                    }
                }
            }
            _ => {}
        }

        // Draw error if present
        if let Some(ref error) = ai_palette.error {
            let error_y = palette_y + ((palette_height - 2) as f32 * cell_height as f32);
            for (i, ch) in error.chars().enumerate() {
                if i >= max_content_width {
                    break;
                }
                self.text_renderer.queue_char(
                    ch,
                    content_x + (i as f32 * cell_width as f32),
                    error_y,
                    Color::Rgb(255, 100, 100),
                    bg_color,
                    false,
                    false,
                    false,
                    false,
                )?;
            }
        }

        Ok(())
    }

    fn render_status_bar(
        &mut self,
        status_bar: &StatusBar,
        cell_width: u32,
        cell_height: u32,
    ) -> Result<(), ui::renderer::RendererError> {
        use terminal::parser::Color;

        if !status_bar.is_visible() {
            return Ok(());
        }

        // Calculate status bar position (bottom of screen)
        let surface_height = self.config.height;
        let status_bar_y = (surface_height - cell_height) as f32;

        // Status bar colors
        let bg_color = Color::Rgb(40, 44, 52); // Dark blue-gray
        let text_color = Color::Rgb(171, 178, 191); // Light gray
        let branch_color = Color::Rgb(152, 195, 121); // Green for git branch
        let separator_color = Color::Rgb(97, 175, 239); // Blue for separators

        // Draw background
        let bar_width = (self.config.width / cell_width) as usize;
        for col in 0..bar_width {
            let char_x = (col as f32) * (cell_width as f32);
            self.text_renderer.queue_char(
                ' ',
                char_x,
                status_bar_y,
                text_color,
                bg_color,
                false,
                false,
                false,
                false,
            )?;
        }

        // Draw directory
        let mut current_x = 2.0 * cell_width as f32;
        let dir_icon = "📁 ";
        for ch in dir_icon.chars() {
            self.text_renderer.queue_char(
                ch,
                current_x,
                status_bar_y,
                text_color,
                bg_color,
                false,
                false,
                false,
                false,
            )?;
            current_x += cell_width as f32;
        }

        for ch in status_bar.current_dir.chars() {
            if (current_x / cell_width as f32) as usize >= bar_width - 20 {
                break; // Leave space for git branch
            }
            self.text_renderer.queue_char(
                ch,
                current_x,
                status_bar_y,
                text_color,
                bg_color,
                false,
                false,
                false,
                false,
            )?;
            current_x += cell_width as f32;
        }

        // Draw git branch if available
        if let Some(ref branch) = status_bar.git_branch {
            // Add separator
            current_x += cell_width as f32;
            let separator = "│";
            for ch in separator.chars() {
                self.text_renderer.queue_char(
                    ch,
                    current_x,
                    status_bar_y,
                    separator_color,
                    bg_color,
                    false,
                    false,
                    false,
                    false,
                )?;
                current_x += cell_width as f32;
            }

            current_x += cell_width as f32;

            // Draw git icon
            let git_icon = " ";
            for ch in git_icon.chars() {
                self.text_renderer.queue_char(
                    ch,
                    current_x,
                    status_bar_y,
                    branch_color,
                    bg_color,
                    false,
                    false,
                    false,
                    false,
                )?;
                current_x += cell_width as f32;
            }

            // Draw branch name
            for ch in branch.chars() {
                if (current_x / cell_width as f32) as usize >= bar_width - 2 {
                    break;
                }
                self.text_renderer.queue_char(
                    ch,
                    current_x,
                    status_bar_y,
                    branch_color,
                    bg_color,
                    false,
                    false,
                    false,
                    false,
                )?;
                current_x += cell_width as f32;
            }
        }

        Ok(())
    }

    fn draw_pane_borders(
        &mut self,
        bounds: Rect,
        cell_width: u32,
        cell_height: u32,
        is_focused: bool,
    ) -> Result<(), ui::renderer::RendererError> {
        // Warp-style: no visible borders, just subtle spacing
        // The background already separates panes
        Ok(())
    }
        }

impl TerminalApp {
    fn new() -> Self {
        let config = AppConfig::default();

        Self {
            window: None,
            renderer: None,
            layout: None,
            input_handler: InputHandler::new(),
            selection_state: SelectionState::new(),
            clipboard: Clipboard::new(),
            running: false,
            last_frame: Instant::now(),
            frame_duration: Duration::from_micros(16_667), // ~60 FPS
            cell_width: 10,
            cell_height: 20,
            cursor_position: None,
            modifiers: ModifiersState::default(),
            search_state: SearchState::new(),
            search_input: String::new(),
            ai_palette: AICommandPalette::new(),
            status_bar: StatusBar::new(),
        }
    }

    /// Create initial pane with PTY
    fn create_initial_pane(&self, cols: u16, rows: u16) -> Result<Pane> {
        let config = PtyConfig {
            cols,
            rows,
            ..Default::default()
        };

        let pty = PtySession::spawn(config)?;
        let bounds = Rect::new(0, 0, cols as u32 * self.cell_width, rows as u32 * self.cell_height);
        
        let mut pane = Pane::new(pty, cols as usize, rows as usize, bounds);
        
        // Add test text to verify rendering
        let test_str = "Hello World! This is a test.";
        for (i, ch) in test_str.chars().enumerate() {
            pane.grid.put_char_at(0, i, ch);
        }
        
        Ok(pane)
    }

    /// Create a new pane with PTY
    fn create_pane(&self, cols: usize, rows: usize, bounds: Rect) -> Result<Pane> {
        let config = PtyConfig {
            cols: cols as u16,
            rows: rows as u16,
            ..Default::default()
        };

        let pty = PtySession::spawn(config)?;
        Ok(Pane::new(pty, cols, rows, bounds))
    }

    /// Read and process PTY output from all panes (non-blocking)
    fn read_all_pty_output(&mut self) {
        if let Some(ref mut layout) = self.layout {
            // Get all pane IDs
            let pane_ids = layout.all_pane_ids();
            
            // Read from each pane
            for pane_id in pane_ids {
                if let Some(pane) = layout.get_pane_mut(pane_id) {
                    Self::read_pane_output(pane);
                }
            }
        }
    }

    /// Read and process PTY output from a single pane
    fn read_pane_output(pane: &mut Pane) {
        // Use async reader on Windows, sync reader on Unix
        #[cfg(target_os = "windows")]
        {
            let data = {
                if let Ok(session) = pane.pty.lock() {
                    session.read_async()
                } else {
                    Vec::new()
                }
            };
            if !data.is_empty() {
                tracing::trace!("read_pane_output: received {} bytes via async", data.len());
                pane.parser.parse_bytes_with_output(&data, &mut pane.grid);
            }
            return;
        }
        
        #[cfg(not(target_os = "windows"))]
        {
            let mut data = Vec::with_capacity(16384);
            let mut has_data = false;
            
            for _ in 0..5 {
                let read_result = {
                    if let Ok(mut session) = pane.pty.lock() {
                        let mut buf = vec![0u8; 4096];
                        match session.read(&mut buf) {
                            Ok(0) => {
                                tracing::info!("PTY closed for pane {}", pane.id);
                                return;
                            }
                            Ok(n) => {
                                buf.truncate(n);
                                (true, Some(buf))
                            }
                            Err(e) => {
                                let err_str = e.to_string();
                                if !err_str.contains("Would block")
                                    && !err_str.contains("Resource temporarily unavailable")
                                {
                                    tracing::debug!("PTY read error: {}", e);
                                }
                                break;
                            }
                        }
                    } else {
                        (false, None)
                    }
                };

                match read_result {
                    (_, Some(buf)) if !buf.is_empty() => {
                        data.extend_from_slice(&buf);
                        has_data = true;
                    }
                    _ => break,
                }
            }

            if has_data && !data.is_empty() {
                Self::process_pane_output(pane, &data);
            }
        }
    }

    /// Process terminal output bytes through the parser to the grid for a specific pane.
    fn process_pane_output(pane: &mut Pane, data: &[u8]) {
        // Sync grid colors/attributes from parser state before processing
        pane.grid.set_foreground(pane.parser.state.fg_color);
        pane.grid.set_background(pane.parser.state.bg_color);
        pane.grid.set_attributes(pane.parser.state.attributes);

        // Use batch mode for grid updates to reduce overhead
        pane.grid.begin_batch();

        // Parse bytes and output directly to the grid
        pane.parser.parse_bytes_with_output(data, &mut pane.grid);

        // Flush batched updates
        pane.grid.flush_batch();
    }

    /// Send input to the focused pane's PTY
    fn send_pty_input(&mut self, data: &[u8]) {
        if !data.is_empty() {
            if let Some(ref layout) = self.layout {
                let focused_id = layout.focused_pane_id();
                if let Some(ref mut layout) = self.layout {
                    if let Some(pane) = layout.get_pane_mut(focused_id) {
                        if let Ok(mut session) = pane.pty.lock() {
                            if let Err(e) = session.write(data) {
                                tracing::error!("Failed to write to PTY: {}", e);
                            }
                        }
                    }
                }
            }
        }
    }

    /// Handle window resize
    fn handle_resize(&mut self, width: u32, height: u32) {
        // Resize the renderer
        if let Some(ref mut renderer) = self.renderer {
            renderer.resize(width, height);
        }

        // Calculate layout bounds
        let total_bounds = Rect::new(0, 0, width, height);

        // Update layout and recalculate all pane bounds
        if let Some(ref mut layout) = self.layout {
            layout.calculate_layout(total_bounds);

            // Resize each pane's grid and PTY based on new bounds
            let pane_ids = layout.all_pane_ids();
            for pane_id in pane_ids {
                if let Some(pane) = layout.get_pane_mut(pane_id) {
                    let (new_cols, new_rows) = pane.terminal_size(self.cell_width, self.cell_height);

                    if new_cols > 0 && new_rows > 0 {
                        pane.grid.resize(new_cols, new_rows);
                        pane.parser.resize(new_cols, new_rows);

                        // Resize the PTY
                        if let Ok(mut session) = pane.pty.lock() {
                            if let Err(e) = session.resize(new_cols as u16, new_rows as u16) {
                                tracing::error!("Failed to resize PTY: {}", e);
                            }
                        }
                    }
                }
            }
        }
    }

    /// Render a frame
    fn render(&mut self) {
        tracing::debug!("render() called");

        // Update AI palette state (check for async responses)
        self.ai_palette.update();

        // Update status bar with focused pane's working directory
        self.update_status_bar();

        if let (Some(ref mut renderer), Some(ref layout)) = (&mut self.renderer, &self.layout) {
            let focused_id = layout.focused_pane_id();
            tracing::debug!("calling render_layout, focused_id={}", focused_id);
            if let Err(e) = renderer.render_layout(
                layout,
                self.cell_width,
                self.cell_height,
                focused_id,
                &self.search_state,
                &self.search_input,
                &self.ai_palette,
                &self.status_bar,
            ) {
                tracing::error!("Render error: {}", e);
            }
        }
    }

    /// Update status bar with current directory and git info
    fn update_status_bar(&mut self) {
        if let Some(ref layout) = self.layout {
            let focused_id = layout.focused_pane_id();
            if let Some(pane) = layout.get_pane(focused_id) {
                // Try to get the working directory from the PTY's child process
                let cwd = self.get_pane_cwd(pane);
                self.status_bar.update(&cwd);
            }
        }
    }

    /// Get the current working directory of a pane's PTY process
    fn get_pane_cwd(&self, pane: &Pane) -> String {
        // First check if we have a tracked directory from OSC 7 shell integration
        if let Some(cwd) = pane.parser.get_current_directory() {
            return cwd.to_string();
        }

        // Fall back to the initial directory
        std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| "~".to_string())
    }

    /// Convert pixel position to grid coordinates (and pane ID)
    fn pixel_to_pane_and_grid(&self, x: f64, y: f64) -> Option<(uuid::Uuid, usize, usize)> {
        if let Some(ref layout) = self.layout {
            let pane_ids = layout.all_pane_ids();
            for pane_id in pane_ids {
                if let Some(pane) = layout.get_pane(pane_id) {
                    if pane.bounds.contains(x as u32, y as u32) {
                        // Click is within this pane
                        let local_x = x as u32 - pane.bounds.x;
                        let local_y = y as u32 - pane.bounds.y;
                        let col = (local_x / self.cell_width) as usize;
                        let row = (local_y / self.cell_height) as usize;

                        if col < pane.grid.cols() && row < pane.grid.rows() {
                            return Some((pane_id, col, row));
                        }
                    }
                }
            }
        }
        None
    }

    /// Handle mouse button press
    fn handle_mouse_button(
        &mut self,
        _device_id: DeviceId,
        button: MouseButton,
        state: ElementState,
    ) {
        // Only handle left mouse button for selection
        if button != MouseButton::Left {
            return;
        }

        if let Some(pos) = self.cursor_position {
            if let Some((pane_id, col, row)) = self.pixel_to_pane_and_grid(pos.x, pos.y) {
                // Focus the clicked pane
                if let Some(ref mut layout) = self.layout {
                    layout.set_focus(pane_id);
                }

                use crate::terminal::grid::Cursor;

                match state {
                    ElementState::Pressed => {
                        // Start selection
                        self.selection_state.start_selection(Cursor::new(row, col));
                    }
                    ElementState::Released => {
                        // End selection
                        self.selection_state.end_selection();

                        // If Shift is held, copy to clipboard
                        if self.modifiers.shift_key() && self.selection_state.has_selection() {
                            // Get the focused pane's grid
                            if let Some(ref layout) = self.layout {
                                if let Some(pane) = layout.focused_pane() {
                                    let selected_text = extract_selected_text(
                                        pane.grid.as_rows(),
                                        &self.selection_state.region,
                                    );
                                    if !selected_text.is_empty() {
                                        if let Err(e) = self.clipboard.copy(&selected_text) {
                                            tracing::warn!("Failed to copy to clipboard: {}", e);
                                        } else {
                                            tracing::debug!("Copied selection to clipboard");
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            } else {
                // Clicked outside grid, clear selection
                self.selection_state.clear();
            }
        }
    }

    /// Handle mouse motion
    fn handle_mouse_motion(&mut self, position: PhysicalPosition<f64>) {
        // Update stored cursor position
        self.cursor_position = Some(position);

        // Only update selection if we're currently selecting
        if self.selection_state.selecting {
            if let Some((_pane_id, col, row)) = self.pixel_to_pane_and_grid(position.x, position.y) {
                use crate::terminal::grid::Cursor;
                self.selection_state.update_selection(Cursor::new(row, col));
            }
        }
    }

    /// Handle paste from clipboard (Ctrl+V or Shift+Insert)
    fn handle_paste(&mut self) -> Result<()> {
        if let Ok(text) = self.clipboard.paste() {
            // Convert text to bytes and send to PTY
            let bytes = text.as_bytes();
            if !bytes.is_empty() {
                self.send_pty_input(bytes);
                tracing::debug!("Pasted {} bytes from clipboard", bytes.len());
            }
        }
        Ok(())
    }

    /// Handle copy to clipboard (Ctrl+Shift+C)
    fn handle_copy(&mut self) {
        // Check if there's a selection
        if self.selection_state.has_selection() {
            if let Some(ref layout) = self.layout {
                if let Some(pane) = layout.focused_pane() {
                    let selected_text = extract_selected_text(
                        pane.grid.as_rows(),
                        &self.selection_state.region,
                    );
                    if !selected_text.is_empty() {
                        if let Err(e) = self.clipboard.copy(&selected_text) {
                            tracing::warn!("Failed to copy to clipboard: {}", e);
                        } else {
                            tracing::debug!("Copied selection to clipboard (Ctrl+Shift+C)");
                        }
                    }
                }
            }
        }
    }

    /// Handle pane split request
    fn handle_split(&mut self, direction: SplitDirection) {
        // Get focused pane info first to avoid borrow conflicts
        let pane_info = if let Some(ref layout) = self.layout {
            if let Some(focused) = layout.focused_pane() {
                let (cols, rows) = focused.terminal_size(self.cell_width, self.cell_height);
                let new_bounds = Rect::new(0, 0, focused.bounds.width / 2, focused.bounds.height);
                Some((cols, rows, new_bounds))
            } else {
                None
            }
        } else {
            None
        };

        // Create new pane if we got info
        if let Some((cols, rows, new_bounds)) = pane_info {
            match self.create_pane(cols, rows, new_bounds) {
                Ok(new_pane) => {
                    // Now do the split
                    if let Some(ref mut layout) = self.layout {
                        if let Err(e) = layout.split_focused(direction, new_pane) {
                            tracing::warn!("Failed to split pane: {}", e);
                        } else {
                            // Recalculate layout
                            if let Some(ref window) = self.window {
                                let size = window.inner_size();
                                self.handle_resize(size.width, size.height);
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to create new pane: {}", e);
                }
            }
        }
    }

    /// Handle focus navigation
    fn handle_focus_next(&mut self) {
        if let Some(ref mut layout) = self.layout {
            layout.focus_next();
        }
    }

    fn handle_focus_prev(&mut self) {
        if let Some(ref mut layout) = self.layout {
            layout.focus_prev();
        }
    }

    /// Toggle search mode
    fn handle_toggle_search(&mut self) {
        self.search_state.active = !self.search_state.active;
        if self.search_state.active {
            self.search_input.clear();
            self.search_state.clear();
        }
    }

    /// Handle search input
    fn handle_search_input(&mut self, c: char) {
        if self.search_state.active {
            self.search_input.push(c);
            self.update_search();
        }
    }

    /// Handle search backspace
    fn handle_search_backspace(&mut self) {
        if self.search_state.active && !self.search_input.is_empty() {
            self.search_input.pop();
            self.update_search();
        }
    }

    /// Update search with current input
    fn update_search(&mut self) {
        if self.search_state.set_pattern(&self.search_input).is_ok() {
            // Find matches in focused pane
            if let Some(ref layout) = self.layout {
                if let Some(pane) = layout.focused_pane() {
                    // Collect all rows from the grid
                    let rows: Vec<(usize, String)> = (0..pane.grid.rows())
                        .map(|row| {
                            let mut line = String::new();
                            for col in 0..pane.grid.cols() {
                                if let Some(cell) = pane.grid.get_cell(row, col) {
                                    line.push(cell.char);
                                } else {
                                    line.push(' ');
                                }
                            }
                            (row, line)
                        })
                        .collect();

                    // Update search state with matches
                    self.search_state.find_matches(rows.iter().map(|(r, l)| (*r, l.as_str())));
                }
            }
        }
    }

    /// Handle search navigation (next match)
    fn handle_search_next(&mut self) {
        if self.search_state.active {
            self.search_state.next_match();
        }
    }

    /// Handle search navigation (previous match)
    fn handle_search_prev(&mut self) {
        if self.search_state.active {
            self.search_state.prev_match();
        }
    }

    /// Close search
    fn handle_search_close(&mut self) {
        self.search_state.active = false;
        self.search_input.clear();
        self.search_state.clear();
    }

    /// Handle pane close (Ctrl+W)
    fn handle_close_pane(&mut self) {
        if let Some(ref mut layout) = self.layout {
            if let Err(e) = layout.close_focused() {
                tracing::warn!("Failed to close pane: {}", e);
            } else {
                // Recalculate layout after closing
                if let Some(ref window) = self.window {
                    let size = window.inner_size();
                    self.handle_resize(size.width, size.height);
                }
            }
        }
    }

    /// Handle pane resize (Ctrl+Shift+Arrow keys)
    fn handle_resize_pane(&mut self, direction: SplitDirection, delta: f32) {
        if let Some(ref mut layout) = self.layout {
            if let Err(e) = layout.resize_focused(direction, delta) {
                tracing::warn!("Failed to resize pane: {}", e);
            } else {
                // Recalculate layout after resizing
                if let Some(ref window) = self.window {
                    let size = window.inner_size();
                    self.handle_resize(size.width, size.height);
                }
            }
        }
    }
}

impl ApplicationHandler for TerminalApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // Create window
        let window = match event_loop.create_window(
            Window::default_attributes()
                .with_title("Warp FOSS")
                .with_inner_size(PhysicalSize::new(1200, 800)),
        ) {
            Ok(w) => Arc::new(w),
            Err(e) => {
                tracing::error!("Failed to create window: {}", e);
                event_loop.exit();
                return;
            }
        };

        // Get initial size
        let size = window.inner_size();
        let cols = (size.width / self.cell_width) as u16;
        let rows = (size.height / self.cell_height) as u16;

        // Initialize renderer
        let renderer = match pollster::block_on(RendererHolder::new(Arc::clone(&window))) {
            Ok(r) => r,
            Err(e) => {
                tracing::error!("Failed to initialize renderer: {}", e);
                event_loop.exit();
                return;
            }
        };

        // Create initial pane and layout
        let initial_pane = match self.create_initial_pane(cols.max(40), rows.max(10)) {
            Ok(p) => p,
            Err(e) => {
                tracing::error!("Failed to create initial pane: {}", e);
                event_loop.exit();
                return;
            }
        };

        let layout = LayoutTree::new(initial_pane);

        self.window = Some(window);
        self.renderer = Some(renderer);
        self.layout = Some(layout);
        self.running = true;

        // Initialize clipboard (must be done from main thread)
        if let Err(e) = self.clipboard.init() {
            tracing::warn!("Failed to initialize clipboard: {}", e);
        }

        tracing::info!("Terminal application started with layout support");
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                self.running = false;
                event_loop.exit();
            }

            WindowEvent::Resized(physical_size) => {
                self.handle_resize(physical_size.width, physical_size.height);
            }

            WindowEvent::KeyboardInput { event, .. } => {
                // Check for special shortcuts
                if event.state == ElementState::Pressed {
                    // Check for AI palette toggle (Ctrl+Space)
                    let is_ctrl_space = matches!(&event.logical_key, Key::Character(c) if c == " " || c == " ")
                        && self.input_handler.modifiers().ctrl;
                    
                    if is_ctrl_space {
                        self.ai_palette.toggle();
                        return;
                    }
                }

                // Handle AI palette input if open
                if self.ai_palette.is_visible() {
                    use winit::event::ElementState;
                    
                    if event.state == ElementState::Pressed {
                        match &event.logical_key {
                            Key::Named(NamedKey::Escape) => {
                                self.ai_palette.handle_escape();
                            }
                            Key::Named(NamedKey::Enter) => {
                                self.ai_palette.handle_enter();
                            }
                            Key::Named(NamedKey::Backspace) => {
                                self.ai_palette.handle_backspace();
                            }
                            Key::Named(NamedKey::ArrowLeft) => {
                                self.ai_palette.cursor_left();
                            }
                            Key::Named(NamedKey::ArrowRight) => {
                                self.ai_palette.cursor_right();
                            }
                            Key::Character(c) => {
                                for ch in c.chars() {
                                    self.ai_palette.handle_char(ch);
                                }
                            }
                            _ => {}
                        }
                    }
                    return;
                }

                // Handle search mode input if search is active
                if self.search_state.active {
                    if event.state == ElementState::Pressed {
                        match &event.logical_key {
                            Key::Named(NamedKey::Escape) => {
                                self.handle_search_close();
                                return;
                            }
                            Key::Named(NamedKey::Enter) => {
                                if self.input_handler.modifiers().shift {
                                    self.handle_search_prev();
                                } else {
                                    self.handle_search_next();
                                }
                                return;
                            }
                            Key::Named(NamedKey::Backspace) => {
                                self.handle_search_backspace();
                                return;
                            }
                            Key::Character(c) => {
                                for ch in c.chars() {
                                    self.handle_search_input(ch);
                                }
                                return;
                            }
                            _ => {
                                // Ignore other keys in search mode
                                return;
                            }
                        }
                    }
                }

                // Check for other shortcuts
                if event.state == ElementState::Pressed {
                    // Check for search toggle (Ctrl+Shift+F)
                    match &event.logical_key {
                        Key::Character(c) if c == "f" || c == "F" => {
                            let modifiers = self.input_handler.modifiers();
                            if modifiers.ctrl && modifiers.shift {
                                self.handle_toggle_search();
                                return;
                            }
                        }
                        _ => {}
                    }

                    // Check for pane splitting (Ctrl+D for horizontal, Ctrl+Shift+D for vertical)
                    match &event.logical_key {
                        Key::Character(c) if c == "d" || c == "D" => {
                            let modifiers = self.input_handler.modifiers();
                            if modifiers.ctrl {
                                if modifiers.shift {
                                    // Ctrl+Shift+D = Vertical split
                                    self.handle_split(SplitDirection::Vertical);
                                    return;
                                } else {
                                    // Ctrl+D = Horizontal split
                                    self.handle_split(SplitDirection::Horizontal);
                                    return;
                                }
                            }
                        }
                        Key::Character(c) if c == "w" || c == "W" => {
                            let modifiers = self.input_handler.modifiers();
                            if modifiers.ctrl && !modifiers.shift {
                                // Ctrl+W = Close pane
                                self.handle_close_pane();
                                return;
                            }
                        }
                        // Copy with Ctrl+Shift+C
                        Key::Character(c) if c == "c" || c == "C" => {
                            let modifiers = self.input_handler.modifiers();
                            if modifiers.ctrl && modifiers.shift {
                                self.handle_copy();
                                return;
                            }
                        }
                        Key::Named(NamedKey::Tab) => {
                            let modifiers = self.input_handler.modifiers();
                            if modifiers.shift {
                                // Shift+Tab = Focus previous pane
                                self.handle_focus_prev();
                            } else if modifiers.ctrl {
                                // Ctrl+Tab = Focus next pane
                                self.handle_focus_next();
                            }
                            return;
                        }
                        // Pane resizing with Ctrl+Shift+Arrow keys
                        Key::Named(NamedKey::ArrowLeft) => {
                            let modifiers = self.input_handler.modifiers();
                            if modifiers.ctrl && modifiers.shift {
                                self.handle_resize_pane(SplitDirection::Horizontal, -0.05);
                                return;
                            }
                        }
                        Key::Named(NamedKey::ArrowRight) => {
                            let modifiers = self.input_handler.modifiers();
                            if modifiers.ctrl && modifiers.shift {
                                self.handle_resize_pane(SplitDirection::Horizontal, 0.05);
                                return;
                            }
                        }
                        Key::Named(NamedKey::ArrowUp) => {
                            let modifiers = self.input_handler.modifiers();
                            if modifiers.ctrl && modifiers.shift {
                                self.handle_resize_pane(SplitDirection::Vertical, -0.05);
                                return;
                            }
                        }
                        Key::Named(NamedKey::ArrowDown) => {
                            let modifiers = self.input_handler.modifiers();
                            if modifiers.ctrl && modifiers.shift {
                                self.handle_resize_pane(SplitDirection::Vertical, 0.05);
                                return;
                            }
                        }
                        _ => {}
                    }

                    // Check for paste shortcuts (Ctrl+V or Ctrl+Shift+V)
                    let is_paste = match &event.logical_key {
                        Key::Named(NamedKey::Paste) => true,
                        Key::Character(c) if c == "v" || c == "V" => {
                            let modifiers = self.input_handler.modifiers();
                            // Support both Ctrl+V and Ctrl+Shift+V
                            modifiers.ctrl
                        }
                        Key::Character(c) if c == "i" || c == "I" => {
                            self.input_handler.modifiers().shift
                        }
                        _ => false,
                    };

                    if is_paste {
                        let _ = self.handle_paste();
                    } else {
                        // Normal input
                        let input = self.input_handler.handle_key_event(&event);
                        let data = input.to_bytes();
                        self.send_pty_input(&data);
                    }
                }
            }

            WindowEvent::ModifiersChanged(modifiers) => {
                self.input_handler
                    .modifiers_mut()
                    .update_from_state(modifiers.state());
                self.modifiers = modifiers.state();
            }

            WindowEvent::MouseInput { state, button, .. } => {
                self.handle_mouse_button(DeviceId::dummy(), button, state);
            }

            WindowEvent::CursorMoved { position, .. } => {
                self.handle_mouse_motion(position);
            }

            WindowEvent::RedrawRequested => {
                tracing::debug!("RedrawRequested received");
                // Read and process any pending PTY output from all panes
                self.read_all_pty_output();

                // Render
                self.render();

                // Request next frame
                if let Some(ref window) = self.window {
                    window.request_redraw();
                }
            }

            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        tracing::debug!("about_to_wait() START, running={}", self.running);

        // Process PTY output periodically
        tracing::debug!("about_to_wait: calling read_all_pty_output");
        self.read_all_pty_output();
        tracing::debug!("about_to_wait: read_all_pty_output done");

        // Limit frame rate
        let elapsed = self.last_frame.elapsed();
        if elapsed < self.frame_duration {
            let wait = self.frame_duration - elapsed;
            std::thread::sleep(wait.min(Duration::from_millis(1)));
        }
        self.last_frame = Instant::now();

        // Request redraw if running
        if self.running {
            if let Some(ref window) = self.window {
                tracing::debug!("about_to_wait: requesting redraw");
                window.request_redraw();
                tracing::debug!("about_to_wait: redraw requested");
            }
        }

        tracing::debug!("about_to_wait() END, setting Poll");
        // Use Poll instead of Wait for continuous rendering on Windows
        event_loop.set_control_flow(ControlFlow::Poll);
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        tracing::info!("Terminal application exiting");
    }
}

fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    tracing::info!("Warp FOSS v0.1.0");
    tracing::info!("Starting terminal application with split pane support...");

    // Create event loop
    let event_loop = EventLoop::new()?;

    // Create app
    let mut app = TerminalApp::new();

    // Run event loop
    event_loop.run_app(&mut app)?;

    Ok(())
}

// Windows-specific: Increase stack size to 8MB to prevent overflow
// This is needed because winit's EventLoop::new() uses significant stack
// on Windows due to deep Windows API call chains (RegisterClassExW, etc.)
#[cfg(all(target_os = "windows", target_env = "gnu"))]
#[link_section = ".stack"]
static STACK_SIZE: u32 = 8 * 1024 * 1024;
