import init, { loxRun } from "lox-wasm";

onmessage = async (event) => {
  await init();
  loxRun(event.data);
};
