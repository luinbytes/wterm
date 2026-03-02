//! WASM plugin system for terminal emulation
//!
//! This module provides a minimal, sandboxed plugin architecture using wasmtime.
//! Plugins can hook into terminal I/O but have no access to filesystem, network,
//! or environment variables.

pub mod manager;

use anyhow::Result;

/// Plugin context passed to hooks
#[derive(Debug, Clone)]
pub struct PluginContext {
    /// Current working directory (if available)
    pub cwd: Option<String>,
    /// Terminal dimensions
    pub cols: u16,
    pub rows: u16,
}

/// Result from a plugin hook
#[derive(Debug, Clone, Default)]
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
pub struct PluginMetadata {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: Option<String>,
    pub description: Option<String>,
}
