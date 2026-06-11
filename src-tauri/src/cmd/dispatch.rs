// Plugin command dispatcher.
//
// Each command module (ports, env, ...) implements `register(&mut CommandRegistry)`
// to plug its handlers into a single routing table. The `dispatch` Tauri command
// is the only entry point exposed to the frontend for plugin-owned commands;
// system commands (windows, settings, ...) stay on the static `invoke_handler!`
// list in lib.rs.
//
// Trade-off: a single dynamic dispatch loses per-command type checking at the
// IPC boundary, but it eliminates the need to edit lib.rs and the codegen
// surface for every new plugin command. The TS side still gets typed wrappers
// from `scripts/codegen-api.ts` reading plugin.json's `commands` array.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use serde_json::Value;

pub type HandlerResult = Result<Value, String>;
pub type Handler = Arc<dyn Fn(Value) -> HandlerResult + Send + Sync>;

/// In-memory routing table. Constructed once in `lib.rs::setup` and shared
/// across all webviews via `app.manage(...)`.
pub struct CommandRegistry {
    handlers: HashMap<&'static str, Handler>,
}

impl CommandRegistry {
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    pub fn register<F>(&mut self, name: &'static str, handler: F)
    where
        F: Fn(Value) -> HandlerResult + Send + Sync + 'static,
    {
        self.handlers.insert(name, Arc::new(handler));
    }

    pub fn dispatch(&self, name: &str, args: Value) -> HandlerResult {
        let handler = self
            .handlers
            .get(name)
            .ok_or_else(|| format!("unknown command: {name}"))?;
        handler(args)
    }

    /// Number of registered commands. Useful for startup sanity checks.
    pub fn len(&self) -> usize {
        self.handlers.len()
    }
}

impl Default for CommandRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Build the routing table by asking each command module to register itself.
/// New plugin = add a `pub mod foo;` in cmd/mod.rs and one line here.
pub fn build_registry(app_state: &crate::AppState) -> CommandRegistry {
    let mut r = CommandRegistry::new();
    super::ports::register(&mut r, app_state.scanner.clone());
    super::env::register(&mut r);
    super::echo::register(&mut r);
    r
}

#[tauri::command]
pub fn dispatch(
    state: tauri::State<'_, Arc<Mutex<CommandRegistry>>>,
    name: String,
    args: Option<Value>,
) -> HandlerResult {
    let args = args.unwrap_or(Value::Null);
    let reg = state
        .lock()
        .map_err(|e| format!("registry lock poisoned: {e}"))?;
    reg.dispatch(&name, args)
}

/// Decode the `args` payload sent by the frontend into a typed struct.
/// Replaces the verbose `serde_json::from_value(args).map_err(|e| e.to_string())?`
/// pattern in every dispatch wrapper.
pub fn parse_args<T: serde::de::DeserializeOwned>(args: Value) -> Result<T, String> {
    serde_json::from_value(args).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn register_and_dispatch_returns_value() {
        let mut r = CommandRegistry::new();
        r.register("ping", |_args| Ok(json!({ "pong": true })));
        let v = r.dispatch("ping", json!({})).unwrap();
        assert_eq!(v, json!({ "pong": true }));
    }

    #[test]
    fn dispatch_unknown_command_errors() {
        let r = CommandRegistry::new();
        let err = r.dispatch("nope", json!({})).unwrap_err();
        assert!(err.contains("unknown command: nope"), "got: {err}");
    }

    #[test]
    fn register_overwrites_with_same_name() {
        // Last writer wins; in practice names are unique.
        let mut r = CommandRegistry::new();
        r.register("dup", |_| Ok(json!("first")));
        r.register("dup", |_| Ok(json!("second")));
        let v = r.dispatch("dup", json!(null)).unwrap();
        assert_eq!(v, json!("second"));
    }

    #[test]
    fn dispatcher_passes_args_through() {
        let mut r = CommandRegistry::new();
        r.register("echo", |args| Ok(args));
        let v = r
            .dispatch("echo", json!({ "name": "x", "port": 80 }))
            .unwrap();
        assert_eq!(v, json!({ "name": "x", "port": 80 }));
    }
}
