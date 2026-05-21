//! Browser-side bindings for the `hl` log renderer.
//!
//! The crate exposes a single [`Renderer`] handle to JavaScript. JS owns the fetcher and
//! virtualized list; it asks the renderer to format individual lines into HTML.

use std::cell::RefCell;

use wasm_bindgen::prelude::*;

mod ansi_html;
mod render;

thread_local! {
    static RENDERER: RefCell<Option<render::Renderer>> = const { RefCell::new(None) };
}

/// Install the panic hook so Rust panics surface in the browser console with a useful trace.
#[wasm_bindgen(start)]
pub fn _start() {
    console_error_panic_hook::set_once();
}

/// Initialize the renderer. Safe to call multiple times — subsequent calls are no-ops.
#[wasm_bindgen]
pub fn init() -> Result<(), JsValue> {
    RENDERER.with(|cell| {
        if cell.borrow().is_some() {
            return Ok(());
        }
        let r = render::Renderer::new().map_err(JsValue::from)?;
        *cell.borrow_mut() = Some(r);
        Ok(())
    })
}

/// Render one or more log records (passed as a single `\n`-separated chunk) to HTML.
///
/// The caller must have invoked [`init`] before this. Returns the HTML as a JS string.
#[wasm_bindgen]
pub fn format_line(bytes: &[u8]) -> Result<String, JsValue> {
    RENDERER.with(|cell| {
        let cell = cell.borrow();
        let r = cell
            .as_ref()
            .ok_or_else(|| JsValue::from_str("renderer not initialized; call init() first"))?;
        Ok(r.format(bytes))
    })
}

/// Convert a stand-alone ANSI-styled text fragment to HTML. Exposed so JS can format lines
/// that were rendered server-side or for testing.
#[wasm_bindgen]
pub fn ansi_to_html(text: &str) -> String {
    ansi_html::convert(text.as_bytes())
}
