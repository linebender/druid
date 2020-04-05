# druid WASM examples

This crate generates and builds all necessary files for deploying `druid` examples to the web.

## Building

You will need `cargo` and `wasm-pack` for building the code and a simple
server like [`http`](https://crates.io/crates/https) for serving the web pages.

First build with

```
> wasm-pack build --target web
```

This step has two main functions:

    1. It generates an HTML document for each of the `druid` examples with a script that
       calls the appropriate function in the JavaScript module exposing the raw WASM.
    2. It builds the WASM binary which exposes all functions annotated with `#[wasm_bindgen]`.
    3. It builds the JavaScript module that loads the WASM binary and binds all exposed functions to
       JavaScript functions so they can be called directly from JavaScript.

To preview the build in a web browser, run

```
> http -d html
```

which should start serving the specified folder.

Finally, point your browser to the appropriate localhost url (usually http://localhost:8000) and you
should see a list of HTML documents -- one for each example.

When you make changes to the project, re-run `wasm-pack build --target web` and you can see the changes in your browser when you refresh -- no need to restart `http`.
