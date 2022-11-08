cfg_if::cfg_if! {
    if #[cfg(target_family = "wasm")] {
        use wasm_bindgen::prelude::*;

        #[wasm_bindgen]
        extern "C" {
            #[wasm_bindgen(js_namespace = Date, js_name = now)]
            fn date_now() -> f64;
        }

        pub fn now() -> f64 {
            date_now() / 1000.0
        }
    } else {
        pub fn now() -> f64 {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs_f64()
        }
    }
}
