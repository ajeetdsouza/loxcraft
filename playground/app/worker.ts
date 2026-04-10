import init, { loxRun } from "lox-wasm";

onmessage = async (event) => {
  await init();
  try {
    loxRun(event.data as string);
  } catch (e) {
    const text = `RuntimeError: ${e instanceof Error ? e.message : e}\n`;
    postMessage(JSON.stringify({ type: "Output", text }));
    postMessage(JSON.stringify({ type: "ExitFailure" }));
  }
};
