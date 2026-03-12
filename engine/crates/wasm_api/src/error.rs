//! WASM error types with JsValue conversion.

use wasm_bindgen::JsValue;

/// Error type for WASM API operations.
#[derive(Debug)]
pub struct WasmError {
    pub message: String,
}

impl WasmError {
    pub fn new(msg: impl Into<String>) -> Self {
        Self { message: msg.into() }
    }
}

impl From<WasmError> for JsValue {
    fn from(err: WasmError) -> JsValue {
        JsValue::from_str(&err.message)
    }
}

impl From<serde_json::Error> for WasmError {
    fn from(err: serde_json::Error) -> Self {
        WasmError::new(format!("JSON error: {}", err))
    }
}

impl From<wh40k_game_core::ExecutionError> for WasmError {
    fn from(err: wh40k_game_core::ExecutionError) -> Self {
        WasmError::new(format!("Execution error: {:?}", err))
    }
}

impl std::fmt::Display for WasmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for WasmError {}
