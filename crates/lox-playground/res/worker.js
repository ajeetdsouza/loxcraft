import init, { lox_run } from "./lox.js";
(async () => await init("./lox_bg.wasm"))();

onmessage = (event) => {
  lox_run(event.data);
};
