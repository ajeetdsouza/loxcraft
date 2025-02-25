use std::hint;

#[cfg(target_family = "wasm")]
use wasm_bindgen::prelude::*;

#[cfg(target_family = "wasm")]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = Date, js_name = now)]
    fn date_now() -> f64;
}

#[cfg(target_family = "wasm")]
pub fn now() -> f64 {
    date_now() / 1000.0
}

#[cfg(not(target_family = "wasm"))]
pub fn now() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
}

#[inline(always)]
pub const fn unreachable() -> ! {
    if cfg!(debug_assertions) { unreachable!() } else { unsafe { hint::unreachable_unchecked() } }
}
