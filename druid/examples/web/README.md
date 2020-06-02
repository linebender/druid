# Druid web examples

This crate generates and builds all the necessary files for deploying `druid` examples to the web.

## Building

You will need `cargo` and `wasm-pack` for building the code and a simple
server like [`http`](https://crates.io/crates/https) for serving the web pages.

First build with

```
> wasm-pack build --target web --dev
```

This step has two main functions:

    1. It generates an HTML document for each of the `druid` examples with a script that
       calls the appropriate function in the JavaScript module exposing the raw Wasm.
    2. It builds the Wasm binary which exposes all functions annotated with `#[wasm_bindgen]`.
    3. It builds the JavaScript module that loads the Wasm binary and binds all the exposed
       functions to JavaScript functions so they can be called directly from JavaScript.

To preview the build in a web browser, run

```
> http
```

which should start serving the crate root folder containing `index.html`.

Finally, point your browser to the appropriate localhost url (usually http://localhost:8000) and you
should see a list of HTML documents -- one for each example.

When you make changes to the project, re-run `wasm-pack build --target web --dev` and you can
see the changes in your browser when you refresh -- no need to restart `http`.

## New Examples

New examples that can be built against the web target should have an associated
`impl_example!(<example_name>)` entry added to `lib.rs`. Examples that don't
support the web target should be specified in the `EXCEPTIONS` list defined
at the top of the `build.rs` script.
