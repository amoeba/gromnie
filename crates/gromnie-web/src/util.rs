use wasm_bindgen::JsValue;

pub(crate) fn js_error(err: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&err.to_string())
}
