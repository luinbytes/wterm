//! Plugin manager for loading and running WASM plugins
//!
//! This module implements a sandboxed plugin system using wasmtime.
//! Plugins have NO access to filesystem, network, or environment variables.
//!
//! Currently unused but kept for future plugin system integration.

use crate::plugin::{HookResult, PluginContext, PluginMetadata};
use anyhow::{anyhow, Context, Result};
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;
use std::rc::Rc;
use std::sync::RwLock;
use wasmtime::*;
use wasmtime_wasi::{ResourceTable, WasiCtx, WasiCtxBuilder, WasiView};

/// State for WASM plugin execution
#[allow(dead_code)]
pub struct PluginState {
    /// WASI context (sandboxed - no filesystem/network/env access)
    wasi: WasiCtx,
    /// Resource table for WASI
    table: ResourceTable,
}

impl WasiView for PluginState {
    fn ctx(&mut self) -> &mut WasiCtx {
        &mut self.wasi
    }

    fn table(&mut self) -> &mut ResourceTable {
        &mut self.table
    }
}

/// Loaded plugin data stored in RefCell for interior mutability
#[allow(dead_code)]
struct LoadedPlugin {
    /// Plugin metadata
    metadata: PluginMetadata,
    /// Module (can be shared across instances)
    module: Module,
    /// Instance and store wrapped in RefCell for mutable access
    store: RefCell<Store<PluginState>>,
    /// Instance
    instance: Instance,
    /// Exported on_input function
    on_input: Option<TypedFunc<(i32, i32), i32>>,
    /// Exported on_output function
    on_output: Option<TypedFunc<(i32, i32), i32>>,
    /// Memory for data exchange
    memory: Memory,
}

/// Plugin manager that loads and runs WASM plugins
#[allow(dead_code)]
pub struct PluginManager {
    /// Loaded plugins indexed by ID (uses Rc for single-threaded access)
    plugins: Rc<RefCell<HashMap<String, LoadedPlugin>>>,
    /// Engine for WASM compilation (can be shared)
    engine: Engine,
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(dead_code)]
impl PluginManager {
    /// Create a new plugin manager
    pub fn new() -> Self {
        let mut config = Config::new();
        config
            .cranelift_opt_level(OptLevel::Speed)
            .wasm_bulk_memory(true)
            .wasm_multi_value(true);

        let engine = Engine::new(&config).expect("Failed to create WASM engine");

        Self {
            plugins: Rc::new(RefCell::new(HashMap::new())),
            engine,
        }
    }

    /// Load a plugin from a WASM file
    pub fn load_plugin(&self, path: &Path) -> Result<String> {
        let wasm_bytes = std::fs::read(path)
            .with_context(|| format!("Failed to read WASM file: {:?}", path))?;

        self.load_plugin_from_bytes(&wasm_bytes)
    }

    /// Load a plugin from WASM bytes
    pub fn load_plugin_from_bytes(&self, wasm_bytes: &[u8]) -> Result<String> {
        // Compile module
        let module = Module::new(&self.engine, wasm_bytes)
            .context("Failed to compile WASM module")?;

        // Create sandboxed WASI context (no filesystem, network, or env access)
        let wasi = WasiCtxBuilder::new()
            .inherit_stdio()
            .build();

        let mut store = Store::new(&self.engine, PluginState {
            wasi,
            table: ResourceTable::new(),
        });

        // Create instance with no imports (fully sandboxed)
        let instance = Instance::new(&mut store, &module, &[])
            .context("Failed to instantiate WASM module")?;

        // Get memory for data exchange
        let memory = instance
            .get_memory(&mut store, "memory")
            .ok_or_else(|| anyhow!("WASM module must export 'memory'"))?;

        // Get optional hook functions
        let on_input = instance.get_typed_func::<(i32, i32), i32>(&mut store, "on_input").ok();
        let on_output = instance.get_typed_func::<(i32, i32), i32>(&mut store, "on_output").ok();

        // Get metadata from exports
        let metadata = self.extract_metadata(&mut store, &instance)?;

        let plugin_id = metadata.id.clone();

        let plugin = LoadedPlugin {
            metadata,
            module,
            store: RefCell::new(store),
            instance,
            on_input,
            on_output,
            memory,
        };

        self.plugins.borrow_mut().insert(plugin_id.clone(), plugin);

        Ok(plugin_id)
    }

    /// Extract plugin metadata from WASM exports
    fn extract_metadata(&self, store: &mut Store<PluginState>, instance: &Instance) -> Result<PluginMetadata> {
        let id = self.get_string_export(store, instance, "plugin_id")
            .unwrap_or_else(|_| format!("plugin-{}", uuid::Uuid::new_v4()));

        let name = self.get_string_export(store, instance, "plugin_name")
            .unwrap_or_else(|_| "Unknown Plugin".to_string());

        let version = self.get_string_export(store, instance, "plugin_version")
            .unwrap_or_else(|_| "0.1.0".to_string());

        let author = self.get_string_export(store, instance, "plugin_author").ok();
        let description = self.get_string_export(store, instance, "plugin_description").ok();

        Ok(PluginMetadata {
            id,
            name,
            version,
            author,
            description,
        })
    }

    /// Get a string from a WASM export function
    fn get_string_export(
        &self,
        store: &mut Store<PluginState>,
        instance: &Instance,
        name: &str,
    ) -> Result<String> {
        let func = instance
            .get_typed_func::<(), i32>(&mut *store, name)
            .with_context(|| format!("Failed to get export: {}", name))?;

        let ptr = func.call(&mut *store, ())?;

        // Read null-terminated string from memory
        let memory = instance
            .get_memory(&mut *store, "memory")
            .ok_or_else(|| anyhow!("No memory export"))?;

        let mut bytes = Vec::new();
        let data = memory.data(&*store);
        let mut offset = ptr as usize;

        while offset < data.len() {
            let byte = data[offset];
            if byte == 0 {
                break;
            }
            bytes.push(byte);
            offset += 1;
        }

        String::from_utf8(bytes).context("Invalid UTF-8 in plugin string")
    }

    /// Unload a plugin by ID
    pub fn unload_plugin(&self, id: &str) -> Result<()> {
        self.plugins
            .borrow_mut()
            .remove(id)
            .ok_or_else(|| anyhow!("Plugin not found: {}", id))?;
        Ok(())
    }

    /// Load all plugins from a directory
    pub fn load_plugins_from_directory(&self, dir: &Path) -> Result<Vec<String>> {
        if !dir.exists() {
            std::fs::create_dir_all(dir)
                .with_context(|| format!("Failed to create plugins directory: {:?}", dir))?;
            return Ok(Vec::new());
        }

        let mut loaded = Vec::new();

        for entry in std::fs::read_dir(dir)
            .with_context(|| format!("Failed to read plugins directory: {:?}", dir))?
        {
            let entry = entry?;
            let path = entry.path();

            if path.extension().map(|e| e == "wasm").unwrap_or(false) {
                match self.load_plugin(&path) {
                    Ok(id) => {
                        tracing::info!("Loaded plugin: {} from {:?}", id, path);
                        loaded.push(id);
                    }
                    Err(e) => {
                        tracing::error!("Failed to load plugin {:?}: {}", path, e);
                    }
                }
            }
        }

        Ok(loaded)
    }

    /// Get list of loaded plugin IDs
    pub fn list_plugins(&self) -> Result<Vec<String>> {
        Ok(self.plugins.borrow().keys().cloned().collect())
    }

    /// Get plugin metadata
    pub fn get_plugin_metadata(&self, id: &str) -> Result<PluginMetadata> {
        let plugins = self.plugins.borrow();
        let plugin = plugins.get(id).ok_or_else(|| anyhow!("Plugin not found: {}", id))?;
        Ok(plugin.metadata.clone())
    }

    /// Call on_input hook on all plugins
    pub fn on_input(&self, data: &[u8], ctx: &PluginContext) -> Result<HookResult> {
        let plugins = self.plugins.borrow();
        let mut result = HookResult::default();
        let mut current_data = data.to_vec();

        for plugin in plugins.values() {
            if let Some(ref on_input) = plugin.on_input {
                let hook_result = self.call_hook(plugin, on_input, &current_data, ctx)?;

                if let Some(ref modified) = hook_result.data {
                    current_data = modified.clone();
                    result.data = Some(modified.clone());
                }
                if hook_result.suppress {
                    result.suppress = true;
                }
            }
        }

        Ok(result)
    }

    /// Call on_output hook on all plugins
    pub fn on_output(&self, data: &[u8], ctx: &PluginContext) -> Result<HookResult> {
        let plugins = self.plugins.borrow();
        let mut result = HookResult::default();
        let mut current_data = data.to_vec();

        for plugin in plugins.values() {
            if let Some(ref on_output) = plugin.on_output {
                let hook_result = self.call_hook(plugin, on_output, &current_data, ctx)?;

                if let Some(ref modified) = hook_result.data {
                    current_data = modified.clone();
                    result.data = Some(modified.clone());
                }
                if hook_result.suppress {
                    result.suppress = true;
                }
            }
        }

        Ok(result)
    }

    /// Call a hook function in a WASM plugin
    fn call_hook(
        &self,
        plugin: &LoadedPlugin,
        func: &TypedFunc<(i32, i32), i32>,
        data: &[u8],
        _ctx: &PluginContext,
    ) -> Result<HookResult> {
        let mut store = plugin.store.borrow_mut();
        let data_ptr = 0u32;

        // Ensure memory has enough space
        let memory_size = plugin.memory.data_size(&*store);
        let required_size = data.len() + 1024;

        if memory_size < required_size {
            let pages_needed = ((required_size - memory_size) / 65536) + 1;
            plugin.memory.grow(&mut *store, pages_needed as u64)?;
        }

        // Copy data to WASM memory
        {
            let memory_data = plugin.memory.data_mut(&mut *store);
            memory_data[..data.len()].copy_from_slice(data);
        }

        // Call the hook function
        let result_len = func.call(&mut *store, (data_ptr as i32, data.len() as i32))?;

        // Read result from WASM memory
        let mut result = HookResult::default();
        if result_len > 0 {
            let memory_data = plugin.memory.data(&*store);
            let len = result_len as usize;
            if len <= memory_data.len() {
                result.data = Some(memory_data[..len].to_vec());
            }
        }

        Ok(result)
    }
}

/// Thread-safe plugin manager for use across threads
#[allow(dead_code)]
pub struct ThreadSafePluginManager {
    plugins: RwLock<HashMap<String, ThreadSafePlugin>>,
    engine: Engine,
}

#[allow(dead_code)]
struct ThreadSafePlugin {
    metadata: PluginMetadata,
    module: Module,
    on_input: Option<Func>,
    on_output: Option<Func>,
    memory_export: String,
}

impl Default for ThreadSafePluginManager {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(dead_code)]
impl ThreadSafePluginManager {
    pub fn new() -> Self {
        let mut config = Config::new();
        config
            .cranelift_opt_level(OptLevel::Speed)
            .wasm_bulk_memory(true);

        let engine = Engine::new(&config).expect("Failed to create WASM engine");

        Self {
            plugins: RwLock::new(HashMap::new()),
            engine,
        }
    }

    /// Load plugin from bytes (stores module for later instantiation)
    pub fn load_plugin_from_bytes(&self, wasm_bytes: &[u8]) -> Result<String> {
        let module = Module::new(&self.engine, wasm_bytes)
            .context("Failed to compile WASM module")?;

        // Extract metadata by creating a temporary instance
        let wasi = WasiCtxBuilder::new().inherit_stdio().build();
        let mut store = Store::new(&self.engine, PluginState {
            wasi,
            table: ResourceTable::new(),
        });

        let instance = Instance::new(&mut store, &module, &[])?;
        let memory_export = instance
            .get_memory(&mut store, "memory")
            .ok_or_else(|| anyhow!("WASM module must export 'memory'"))?;

        // Check if memory is exported
        let _ = memory_export;

        let on_input = instance.get_func(&mut store, "on_input");
        let on_output = instance.get_func(&mut store, "on_output");

        // Extract metadata
        let metadata = self.extract_metadata_from_instance(&mut store, &instance)?;

        let plugin_id = metadata.id.clone();

        let plugin = ThreadSafePlugin {
            metadata,
            module,
            on_input,
            on_output,
            memory_export: "memory".to_string(),
        };

        self.plugins.write().map_err(|e| anyhow!("Lock error: {}", e))?
            .insert(plugin_id.clone(), plugin);

        Ok(plugin_id)
    }

    fn extract_metadata_from_instance(
        &self,
        store: &mut Store<PluginState>,
        instance: &Instance,
    ) -> Result<PluginMetadata> {
        let get_string = |store: &mut Store<PluginState>, name: &str| -> Option<String> {
            let func = instance.get_typed_func::<(), i32>(&mut *store, name).ok()?;
            let ptr = func.call(&mut *store, ()).ok()?;
            let memory = instance.get_memory(&mut *store, "memory")?;

            let mut bytes = Vec::new();
            let data = memory.data(&*store);
            let mut offset = ptr as usize;

            while offset < data.len() {
                let byte = data[offset];
                if byte == 0 {
                    break;
                }
                bytes.push(byte);
                offset += 1;
            }

            String::from_utf8(bytes).ok()
        };

        let id = get_string(store, "plugin_id")
            .unwrap_or_else(|| format!("plugin-{}", uuid::Uuid::new_v4()));
        let name = get_string(store, "plugin_name")
            .unwrap_or_else(|| "Unknown Plugin".to_string());
        let version = get_string(store, "plugin_version")
            .unwrap_or_else(|| "0.1.0".to_string());
        let author = get_string(store, "plugin_author");
        let description = get_string(store, "plugin_description");

        Ok(PluginMetadata {
            id,
            name,
            version,
            author,
            description,
        })
    }

    pub fn list_plugins(&self) -> Result<Vec<String>> {
        Ok(self.plugins.read().map_err(|e| anyhow!("Lock error: {}", e))?
            .keys().cloned().collect())
    }

    pub fn unload_plugin(&self, id: &str) -> Result<()> {
        self.plugins.write().map_err(|e| anyhow!("Lock error: {}", e))?
            .remove(id)
            .ok_or_else(|| anyhow!("Plugin not found: {}", id))?;
        Ok(())
    }

    pub fn get_plugin_metadata(&self, id: &str) -> Result<PluginMetadata> {
        let plugins = self.plugins.read().map_err(|e| anyhow!("Lock error: {}", e))?;
        let plugin = plugins.get(id).ok_or_else(|| anyhow!("Plugin not found: {}", id))?;
        Ok(plugin.metadata.clone())
    }

    /// Execute on_input on a plugin (creates new instance per call for thread safety)
    pub fn on_input(&self, data: &[u8], _ctx: &PluginContext) -> Result<HookResult> {
        let plugins = self.plugins.read().map_err(|e| anyhow!("Lock error: {}", e))?;
        let mut result = HookResult::default();

        for plugin in plugins.values() {
            if plugin.on_input.is_some() {
                let hook_result = self.execute_hook(plugin, data)?;
                if let Some(modified) = hook_result.data {
                    result.data = Some(modified);
                }
                if hook_result.suppress {
                    result.suppress = true;
                }
            }
        }

        Ok(result)
    }

    /// Execute on_output on a plugin (creates new instance per call for thread safety)
    pub fn on_output(&self, data: &[u8], _ctx: &PluginContext) -> Result<HookResult> {
        let plugins = self.plugins.read().map_err(|e| anyhow!("Lock error: {}", e))?;
        let mut result = HookResult::default();

        for plugin in plugins.values() {
            if plugin.on_output.is_some() {
                let hook_result = self.execute_hook(plugin, data)?;
                if let Some(modified) = hook_result.data {
                    result.data = Some(modified);
                }
                if hook_result.suppress {
                    result.suppress = true;
                }
            }
        }

        Ok(result)
    }

    fn execute_hook(&self, plugin: &ThreadSafePlugin, data: &[u8]) -> Result<HookResult> {
        // Create fresh instance for this execution
        let wasi = WasiCtxBuilder::new().inherit_stdio().build();
        let mut store = Store::new(&self.engine, PluginState {
            wasi,
            table: ResourceTable::new(),
        });

        let instance = Instance::new(&mut store, &plugin.module, &[])?;
        let memory = instance
            .get_memory(&mut store, &plugin.memory_export)
            .ok_or_else(|| anyhow!("Memory not found"))?;

        // Get hook function (try both on_input and on_output)
        let func = instance.get_func(&mut store, "on_input")
            .or_else(|| instance.get_func(&mut store, "on_output"))
            .ok_or_else(|| anyhow!("No hook function found"))?;

        let typed_func = func.typed::<(i32, i32), i32>(&store)?;

        // Ensure memory has enough space
        let memory_size = memory.data_size(&store);
        let required_size = data.len() + 1024;
        if memory_size < required_size {
            let pages = ((required_size - memory_size) / 65536) + 1;
            memory.grow(&mut store, pages as u64)?;
        }

        // Copy data to memory
        {
            let mem_data = memory.data_mut(&mut store);
            mem_data[..data.len()].copy_from_slice(data);
        }

        // Call function
        let result_len = typed_func.call(&mut store, (0, data.len() as i32))?;

        // Read result
        let mut result = HookResult::default();
        if result_len > 0 {
            let mem_data = memory.data(&store);
            let len = result_len as usize;
            if len <= mem_data.len() {
                result.data = Some(mem_data[..len].to_vec());
            }
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_manager_creation() {
        let manager = PluginManager::new();
        assert!(manager.list_plugins().unwrap().is_empty());
    }

    #[test]
    fn test_load_minimal_plugin() {
        let wat = r#"
(module
    (memory (export "memory") 1)
    (func (export "plugin_id") (result i32)
        i32.const 0
    )
    (data (i32.const 0) "test-plugin\00")
)
"#;

        let wasm = wat::parse_str(wat).expect("Failed to parse WAT");
        let manager = PluginManager::new();

        let result = manager.load_plugin_from_bytes(&wasm);
        assert!(result.is_ok(), "Failed to load plugin: {:?}", result.err());

        let id = result.unwrap();
        assert_eq!(id, "test-plugin");
    }

    #[test]
    fn test_plugin_metadata_extraction() {
        let wat = r#"
(module
    (memory (export "memory") 1)
    (func (export "plugin_id") (result i32)
        i32.const 0
    )
    (func (export "plugin_name") (result i32)
        i32.const 20
    )
    (func (export "plugin_version") (result i32)
        i32.const 40
    )
    (func (export "plugin_author") (result i32)
        i32.const 50
    )
    (data (i32.const 0) "meta-plugin\00")
    (data (i32.const 20) "Metadata Plugin\00")
    (data (i32.const 40) "1.0.0\00")
    (data (i32.const 50) "Test Author\00")
)
"#;

        let wasm = wat::parse_str(wat).expect("Failed to parse WAT");
        let manager = PluginManager::new();

        let id = manager.load_plugin_from_bytes(&wasm).expect("Failed to load plugin");
        let metadata = manager.get_plugin_metadata(&id).expect("Failed to get metadata");

        assert_eq!(metadata.id, "meta-plugin");
        assert_eq!(metadata.name, "Metadata Plugin");
        assert_eq!(metadata.version, "1.0.0");
        assert_eq!(metadata.author, Some("Test Author".to_string()));
    }

    #[test]
    fn test_unload_plugin() {
        let wat = r#"
(module
    (memory (export "memory") 1)
    (func (export "plugin_id") (result i32)
        i32.const 0
    )
    (data (i32.const 0) "unload-test\00")
)
"#;

        let wasm = wat::parse_str(wat).expect("Failed to parse WAT");
        let manager = PluginManager::new();

        let id = manager.load_plugin_from_bytes(&wasm).expect("Failed to load plugin");
        assert_eq!(manager.list_plugins().unwrap().len(), 1);

        manager.unload_plugin(&id).expect("Failed to unload");
        assert!(manager.list_plugins().unwrap().is_empty());
    }

    #[test]
    fn test_sandboxed_execution() {
        let wat = r#"
(module
    (memory (export "memory") 1)
    (func (export "plugin_id") (result i32)
        i32.const 0
    )
    (func (export "plugin_name") (result i32)
        i32.const 20
    )
    (data (i32.const 0) "sandboxed\00")
    (data (i32.const 20) "Sandboxed Plugin\00")
)
"#;

        let wasm = wat::parse_str(wat).expect("Failed to parse WAT");
        let manager = PluginManager::new();

        let result = manager.load_plugin_from_bytes(&wasm);
        assert!(result.is_ok(), "Sandboxed plugin should load: {:?}", result.err());
    }

    #[test]
    fn test_load_multiple_plugins() {
        let wat1 = r#"
(module
    (memory (export "memory") 1)
    (func (export "plugin_id") (result i32) i32.const 0)
    (data (i32.const 0) "plugin-one\00")
)
"#;

        let wat2 = r#"
(module
    (memory (export "memory") 1)
    (func (export "plugin_id") (result i32) i32.const 0)
    (data (i32.const 0) "plugin-two\00")
)
"#;

        let wasm1 = wat::parse_str(wat1).expect("Failed to parse WAT");
        let wasm2 = wat::parse_str(wat2).expect("Failed to parse WAT");
        let manager = PluginManager::new();

        let id1 = manager.load_plugin_from_bytes(&wasm1).expect("Failed to load plugin 1");
        let id2 = manager.load_plugin_from_bytes(&wasm2).expect("Failed to load plugin 2");

        let plugins = manager.list_plugins().unwrap();
        assert_eq!(plugins.len(), 2);
        assert!(plugins.contains(&id1));
        assert!(plugins.contains(&id2));
    }

    #[test]
    fn test_plugin_with_invalid_wasm() {
        let invalid_wasm = b"not valid wasm";
        let manager = PluginManager::new();

        let result = manager.load_plugin_from_bytes(invalid_wasm);
        assert!(result.is_err(), "Should fail to load invalid WASM");
    }

    #[test]
    fn test_hook_execution() {
        // Plugin that echoes input with a prefix
        let wat = r#"
(module
    (memory (export "memory") 2)

    (func (export "plugin_id") (result i32)
        i32.const 0
    )
    (func (export "plugin_name") (result i32)
        i32.const 20
    )

    ;; on_input returns 0 (no modification)
    (func (export "on_input") (param i32 i32) (result i32)
        i32.const 0
    )

    ;; on_output returns length of data unchanged
    (func (export "on_output") (param i32 i32) (result i32)
        local.get 1  ;; return the input length
    )

    (data (i32.const 0) "echo-plugin\00")
    (data (i32.const 20) "Echo Plugin\00")
)
"#;

        let wasm = wat::parse_str(wat).expect("Failed to parse WAT");
        let manager = PluginManager::new();

        let _id = manager.load_plugin_from_bytes(&wasm).expect("Failed to load plugin");

        let ctx = PluginContext {
            cwd: Some("/test".to_string()),
            cols: 80,
            rows: 24,
        };

        // Test on_output - should return the same data
        let result = manager.on_output(b"hello", &ctx).expect("on_output failed");
        assert!(result.data.is_some(), "Should have modified data");
        assert_eq!(result.data.unwrap(), b"hello");
    }

    #[test]
    fn test_thread_safe_manager() {
        let manager = ThreadSafePluginManager::new();

        let wat = r#"
(module
    (memory (export "memory") 1)
    (func (export "plugin_id") (result i32) i32.const 0)
    (data (i32.const 0) "thread-safe\00")
)
"#;

        let wasm = wat::parse_str(wat).expect("Failed to parse WAT");
        let id = manager.load_plugin_from_bytes(&wasm).expect("Failed to load");

        assert!(manager.list_plugins().unwrap().contains(&id));
        manager.unload_plugin(&id).expect("Failed to unload");
        assert!(!manager.list_plugins().unwrap().contains(&id));
    }
}
