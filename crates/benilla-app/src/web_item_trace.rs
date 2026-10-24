//! Bounded item-cast diagnostics for the browser's local diagnostic report.

pub(crate) fn record(message: String) {
    let Some(window) = web_sys::window() else {
        return;
    };
    if !js_sys::Reflect::has(&window, &"__wenilla_ui_diagnostic".into()).unwrap_or(false) {
        return;
    }
    let key = wasm_bindgen::JsValue::from_str("__wenilla_item_trace");
    let value = js_sys::Reflect::get(&window, &key).unwrap_or_default();
    let events = if js_sys::Array::is_array(&value) {
        js_sys::Array::from(&value)
    } else {
        js_sys::Array::new()
    };
    events.push(&format!("{:.0} {message}", js_sys::Date::now()).into());
    if events.length() > 32 {
        events.shift();
    }
    let _ = js_sys::Reflect::set(&window, &key, &events);
}
