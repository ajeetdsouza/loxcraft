import init, { lox_run } from "./lox.js";

onmessage = e => {
	var port_stdin = e.ports[0];
	var port_stdout = e.ports[1];
	var port_stderr = e.ports[2];
	(async () => {
		await init("./lox.wasm");
		port_stdin.onmessage = function (stdin_event) {
			lox_run(stdin_event.data, port_stdout, port_stderr);
		}
	})();
};
