# Druid web hello world

This is a minimal example of building a single druid application for the web.
To build all the druid examples for the web, check out the `web` example directory.

## Building

You will need `cargo` and `wasm-pack` for building the code and a simple
server like [`http`](https://crates.io/crates/https) for serving the web page.

First build with.

```
> wasm-pack build --target web --dev
```

This generates a JavaScript module that exports the `wasm_main` function that's
been annotated with the `#[wasm_bindgen]` macro. Leave off the `--dev` flag
if you're doing a release build.

Now run

```
> http
```

which should start serving this directory.

Finally, point your browser to the appropriate localhost url (usually http://localhost:8000) and you
should see your app.

When you make changes to the project, re-run `wasm-pack build --target web --dev` and you can
see the changes in your browser when you refresh -- no need to restart `http`.
