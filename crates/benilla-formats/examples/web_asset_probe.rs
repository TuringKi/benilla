//! Browser regression probe for readable MPQ members omitted from `/data/__index`.
fn main() {}

#[cfg(target_arch = "wasm32")]
thread_local! {
    static CHAIN: benilla_formats::Chain =
        benilla_formats::Chain::open(std::path::Path::new("/data")).unwrap();
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn asset_probe(name: &str) -> String {
    CHAIN.with(|chain| {
        let exists = chain.contains(name);
        let read = chain.read(name);
        serde_json::json!({
            "exists": exists,
            "bytes": read.as_ref().ok().map(Vec::len),
            "error": read.err().map(|e| e.to_string()),
            "url": chain.url_for_name(name),
        })
        .to_string()
    })
}
