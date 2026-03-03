//! WASM plugin system for terminal emulation
//!
//! This module provides a minimal, sandboxed plugin architecture using wasmtime.
//! Plugins can hook into terminal I/O but have no access to filesystem, network,
//! or environment variables.
//!
//! # Example
//!
//! ```ignore
//! use warp_foss::plugin::{PluginManager, PluginContext};
//!
//! let manager = PluginManager::new();
//!
//! // Load plugin from file
//! manager.load_plugin("my_plugin.wasm".as_ref())?;
//!
//! // Or load from bytes
//! manager.load_plugin_from_bytes(&wasm_bytes)?;
//!
//! // Call hooks
//! let ctx = PluginContext {
//!     cwd: Some("/home/user".to_string()),
//!     cols: 80,
//!     rows: 24,
//! };
//!
//! let result = manager.on_output(b"Hello, World!", &ctx)?;
//! if let Some(modified) = result.data {
//!     println!("Modified output: {:?}", String::from_utf8_lossy(&modified));
//! }
//! ```
//!
//! # Plugin API
//!
//! Plugins must export:
//! - `memory` - At least 1 page of memory
//! - `plugin_id` - Function returning pointer to null-terminated ID string
//!
//! Optional exports:
//! - `plugin_name`, `plugin_version`, `plugin_author`, `plugin_description`
//! - `on_input(ptr, len) -> len` - Hook for user input
//! - `on_output(ptr, len) -> len` - Hook for terminal output

pub mod manager;

// Re-export main types for convenience (kept for future use)
#[allow(unused_imports)]
pub use manager::{PluginManager, PluginState, ThreadSafePluginManager};

use anyhow::Result;

/// Plugin context passed to hooks
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct PluginContext {
    /// Current working directory (if available)
    pub cwd: Option<String>,
    /// Terminal dimensions
    pub cols: u16,
    pub rows: u16,
}

/// Result from a plugin hook
#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct HookResult {
    /// Modified data (if the plugin wants to transform it)
    pub data: Option<Vec<u8>>,
    /// Whether to suppress the original event
    pub suppress: bool,
}

/// Trait defining plugin hooks
///
/// Plugins implement this trait to intercept and optionally modify
/// terminal I/O. All hooks are optional - return `None` to pass through.
#[allow(dead_code)]
pub trait Plugin: Send + Sync {
    /// Unique plugin identifier
    fn id(&self) -> &str;

    /// Plugin name for display
    fn name(&self) -> &str;

    /// Called when data is sent to the terminal (user input)
    fn on_input(&self, data: &[u8], ctx: &PluginContext) -> Result<HookResult> {
        let _ = (data, ctx);
        Ok(HookResult::default())
    }

    /// Called when data is received from the terminal (output)
    fn on_output(&self, data: &[u8], ctx: &PluginContext) -> Result<HookResult> {
        let _ = (data, ctx);
        Ok(HookResult::default())
    }
}

/// Plugin metadata loaded from WASM module
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct PluginMetadata {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: Option<String>,
    pub description: Option<String>,
}
