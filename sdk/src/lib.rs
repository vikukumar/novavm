//! # NovaVM SDK
//!
//! The public SDK crate for building NovaVM plugins.
//!
//! Third-party plugins implement the [`NovaPlugin`] trait and are loaded as
//! dynamic libraries via a stable C ABI boundary.
//!
//! ## Example Plugin
//!
//! ```rust,no_run
//! use nova_sdk::{NovaPlugin, PluginContext, PluginError, PluginMetadata};
//!
//! pub struct MyPlugin;
//!
//! impl NovaPlugin for MyPlugin {
//!     fn metadata(&self) -> PluginMetadata {
//!         PluginMetadata {
//!             id: "com.example.my-plugin".to_owned(),
//!             name: "My Plugin".to_owned(),
//!             version: "1.0.0".to_owned(),
//!             description: "An example NovaVM plugin".to_owned(),
//!             author: "Example Author".to_owned(),
//!             min_novavm_version: "0.1.0".to_owned(),
//!         }
//!     }
//!
//!     fn on_load(&mut self, ctx: &PluginContext) -> Result<(), PluginError> {
//!         println!("Plugin loaded!");
//!         Ok(())
//!     }
//!
//!     fn on_unload(&mut self) {}
//! }
//! ```

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Plugin metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    /// Reverse-DNS plugin identifier (e.g. `com.example.my-plugin`).
    pub id: String,
    /// Human-readable plugin name.
    pub name: String,
    /// Plugin version in semver format.
    pub version: String,
    /// Short description of what the plugin does.
    pub description: String,
    /// Plugin author or organisation.
    pub author: String,
    /// Minimum NovaVM version required.
    pub min_novavm_version: String,
}

/// Context passed to the plugin on load, giving it access to registered
/// extension points.
#[derive(Debug)]
pub struct PluginContext {
    /// Unique ID assigned to this plugin instance.
    pub instance_id: Uuid,
    /// NovaVM version running.
    pub novavm_version: String,
    /// Data directory where the plugin may persist its state.
    pub data_dir: std::path::PathBuf,
}

/// Plugin errors.
#[derive(Debug, thiserror::Error)]
pub enum PluginError {
    #[error("Plugin initialisation failed: {0}")]
    InitFailed(String),
    #[error("Plugin hook error: {0}")]
    HookError(String),
    #[error("Plugin I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// The core trait every NovaVM plugin must implement.
///
/// # Safety
/// Plugins are loaded as dynamic libraries. The trait object is passed across
/// the plugin boundary via a stable C ABI wrapper (not included here for brevity).
pub trait NovaPlugin: Send + Sync {
    /// Return static metadata about the plugin.
    fn metadata(&self) -> PluginMetadata;

    /// Called when the plugin is loaded. The plugin should register its hooks
    /// here.
    fn on_load(&mut self, ctx: &PluginContext) -> Result<(), PluginError>;

    /// Called when the plugin is about to be unloaded. Should release all
    /// resources.
    fn on_unload(&mut self);

    /// Called when a VM is created. Optional — default no-op.
    fn on_vm_created(&mut self, _vm_id: Uuid, _vm_name: &str) {}

    /// Called when a VM starts. Optional — default no-op.
    fn on_vm_started(&mut self, _vm_id: Uuid) {}

    /// Called when a VM stops. Optional — default no-op.
    fn on_vm_stopped(&mut self, _vm_id: Uuid) {}

    /// Called when a VM is destroyed. Optional — default no-op.
    fn on_vm_destroyed(&mut self, _vm_id: Uuid) {}
}

/// Plugin registry — holds all loaded plugins.
#[derive(Default)]
pub struct PluginRegistry {
    plugins: Vec<Box<dyn NovaPlugin>>,
}

impl PluginRegistry {
    /// Create an empty plugin registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a plugin.
    pub fn register(&mut self, plugin: Box<dyn NovaPlugin>) {
        // Note: the SDK intentionally avoids a tracing dependency to keep
        // plugins portable. Hosts can log registration themselves.
        self.plugins.push(plugin);
    }

    /// Return all registered plugin metadata.
    pub fn list(&self) -> Vec<PluginMetadata> {
        self.plugins.iter().map(|p| p.metadata()).collect()
    }

    /// Broadcast `on_vm_started` to all plugins.
    pub fn broadcast_vm_started(&mut self, vm_id: Uuid) {
        for plugin in &mut self.plugins {
            plugin.on_vm_started(vm_id);
        }
    }

    /// Broadcast `on_vm_stopped` to all plugins.
    pub fn broadcast_vm_stopped(&mut self, vm_id: Uuid) {
        for plugin in &mut self.plugins {
            plugin.on_vm_stopped(vm_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestPlugin;

    impl NovaPlugin for TestPlugin {
        fn metadata(&self) -> PluginMetadata {
            PluginMetadata {
                id: "com.test.plugin".to_owned(),
                name: "Test".to_owned(),
                version: "0.1.0".to_owned(),
                description: "Test plugin".to_owned(),
                author: "Tester".to_owned(),
                min_novavm_version: "0.1.0".to_owned(),
            }
        }

        fn on_load(&mut self, _ctx: &PluginContext) -> Result<(), PluginError> {
            Ok(())
        }

        fn on_unload(&mut self) {}
    }

    #[test]
    fn test_plugin_registry() {
        let mut registry = PluginRegistry::new();
        registry.register(Box::new(TestPlugin));
        assert_eq!(registry.list().len(), 1);
        assert_eq!(registry.list()[0].id, "com.test.plugin");
    }
}
