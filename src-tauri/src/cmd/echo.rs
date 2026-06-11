// Phase 1 verification plugin: trivial echo command. Demonstrates the
// minimum surface for adding a new backend command module:
//   - pub fn register(&mut CommandRegistry)
//   - dispatch wrapper that decodes args + encodes the result
// Plus the call site (cmd/mod.rs, cmd/dispatch.rs::build_registry) — see
// docs/plugin-loader-phase1.md.

use serde::Serialize;
use serde_json::Value;

#[derive(Serialize)]
pub struct EchoResult {
    pub message: String,
}

pub fn echo_inner(message: &str) -> EchoResult {
    EchoResult {
        message: message.to_string(),
    }
}

pub fn register(r: &mut super::dispatch::CommandRegistry) {
    r.register("echo", |args: Value| -> Result<Value, String> {
        let m = args
            .get("message")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "missing or non-string 'message'".to_string())?;
        let result = echo_inner(m);
        serde_json::to_value(result).map_err(|e| e.to_string())
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn echo_inner_returns_same_string() {
        let r = echo_inner("hi");
        assert_eq!(r.message, "hi");
    }

    #[test]
    fn register_and_dispatch_round_trip() {
        let mut r = super::super::dispatch::CommandRegistry::new();
        register(&mut r);
        let v = r.dispatch("echo", json!({ "message": "hello" })).unwrap();
        assert_eq!(v, json!({ "message": "hello" }));
    }

    #[test]
    fn dispatch_missing_message_errors() {
        let mut r = super::super::dispatch::CommandRegistry::new();
        register(&mut r);
        let err = r.dispatch("echo", json!({})).unwrap_err();
        assert!(err.contains("message"), "got: {err}");
    }
}
