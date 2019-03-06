#!/bin/sh

set -ex

# Run the `wasm-pack` CLI tool to build and process the Rust wasm file
wasm-pack build -d basic-web-static/dist

# Finally, package everything up using Webpack and start a server so we can
# browse the result
cd basic-web-static
npm install
npm run serve
