importScripts("./lox.js");

const { lox_run } = wasm_bindgen;

async function init_wasm_in_worker() {
  await wasm_bindgen("./lox_bg.wasm");
  self.onmessage = (event) => {
    lox_run(event.data);
  };
}
init_wasm_in_worker();
