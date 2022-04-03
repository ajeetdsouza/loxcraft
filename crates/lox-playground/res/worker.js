import init, { lox_run } from "./lox.js";

onmessage = (event) => {
  (async () => await init("./lox_bg.wasm"))();
  const port = event.ports[0];
  port.onmessage = (event) => {
    lox_run(event.data, port);
  };
};
