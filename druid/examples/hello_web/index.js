import init, { wasm_main } from "./pkg/hello_web.js";

async function run() {
  await init();
  wasm_main();
}

run();
