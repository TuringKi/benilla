//! Turtle 1.18 ships its own unified settings and ESC menu. Keep those together instead of
//! layering the stock-1.12 settings adapters over a different FrameXML contract.

use super::addons::Addon;

pub(super) fn layer(mut addon: Addon) -> Addon {
    #[cfg(target_arch = "wasm32")]
    let enabled = crate::webenv::var("WOW_TURTLE_COMPAT").as_deref() == Some("1");
    #[cfg(not(target_arch = "wasm32"))]
    let enabled = std::env::var("WOW_TURTLE_COMPAT").as_deref() == Ok("1");
    if enabled {
        addon
            .toc
            .files
            .retain(|file| !matches!(file.as_str(), "OptionsFrame.xml" | "GameMenuAdapters.xml"));
    }
    addon
}
