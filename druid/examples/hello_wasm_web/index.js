import init, { wasm_main } from "./pkg/hello_wasm_web.js";

async function run() {
  await init();
  wasm_main();
}

run();
