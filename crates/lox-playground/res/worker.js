importScripts("./lox.js");

const { loxRun } = wasm_bindgen;
const loxInit = async () => {
  await wasm_bindgen("./lox_bg.wasm");
  self.onmessage = (event) => {
    loxRun(event.data);
  };
};
loxInit();
