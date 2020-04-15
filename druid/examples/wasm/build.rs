use std::io::Result;
use std::path::PathBuf;
use std::{env, fs};

/// Examples known to not work with WASM are skipped. Ideally this list will eventually be empty.
const EXCEPTIONS: &[&str] = &[
    "svg",       // usvg doesn't currently build with WASM.
    "ext_event", // WASM doesn't currently support spawning threads.
];

fn main() -> Result<()> {
    let crate_dir = PathBuf::from(&env::var("CARGO_MANIFEST_DIR").unwrap());
    let src_dir = crate_dir.join("src");
    let examples_dir = src_dir.join("examples");

    let parent_dir = crate_dir.parent().unwrap();

    // Create a symlink (platform specific) to the examples directory.
    #[cfg(unix)]
    std::os::unix::fs::symlink(parent_dir, &examples_dir).ok();
    #[cfg(windows)]
    std::os::windows::fs::symlink_dir(parent_dir, &examples_dir).ok();

    // Generate example module and the necessary html documents.

    // Declare the newly found example modules in examples.in
    let mut examples_in = r#"
// This file is automatically generated and must not be committed.

/// This is a module collecting all valid examples in the parent examples directory.
mod examples {
"#
    .to_string();

    let mut index_html = r#"
<!DOCTYPE html>
<html lang="en">
    <head>
        <meta charset="utf-8">
        <title>Druid WASM examples - index</title>
    </head>
    <body>
        <h1>Druid WASM examples</h1>
        <ul>
"#
    .to_string();

    for entry in examples_dir.read_dir()? {
        let path = entry?.path();
        if let Some(r) = path.extension() {
            if r != "rs" {
                continue;
            }
        } else {
            continue;
        }

        if let Some(example) = path.file_stem() {
            let example_str = example.to_string_lossy();

            // Skip examples that are known to not work with wasm.
            if EXCEPTIONS.contains(&example_str.as_ref()) {
                continue;
            }

            // Record the valid example module we found to add to the generated examples.in
            examples_in.push_str(&format!("    pub mod {};\n", example_str));

            // The "switch" example name would conflict with JavaScript's switch statement. So we
            // rename it here to switch_demo.
            let js_entry_fn_name = if &example_str == "switch" {
                "switch_demo".to_string()
            } else {
                example_str.to_string()
            };

            // Add an entry to the index.html file.
            let index_entry = format!(
                "<li><a href=\"./html/{name}.html\">{name}</a></li>",
                name = js_entry_fn_name
            );

            index_html.push_str(&index_entry);

            // Create an html document for each example.
            let html = format!(
                r#"
<!DOCTYPE html>
<html lang="en">
    <head>
        <meta charset="utf-8">
        <title>Druid WASM examples - {name}</title>
        <style>
            html, body, canvas {{
                margin: 0px;
                padding: 0px;
                width: 100%;
                height: 100%;
                overflow: hidden;
            }}
        </style>
    </head>
    <body>
        <noscript>This page contains webassembly and javascript content, please enable javascript in your browser.</noscript>
        <canvas id="canvas"></canvas>
        <script type="module">
            import init, {{ {name} }} from '../pkg/druid_wasm_examples.js';

            async function run() {{
                await init();
                {name}();
            }}

            run();
        </script>
    </body>
</html>"#,
                name = js_entry_fn_name
            );

            // Write out the html file into a designated html directory located in crate root.
            let html_dir = crate_dir.join("html");
            if !html_dir.exists() {
                fs::create_dir(&html_dir).unwrap_or_else(|_| {
                    panic!("Failed to create output html directory: {:?}", &html_dir)
                });
            }

            fs::write(html_dir.join(example).with_extension("html"), html)
                .unwrap_or_else(|_| panic!("Failed to create {}.html", example_str));
        }
    }

    examples_in.push_str("}");

    index_html.push_str("</ul></body></html>");

    // Write out the contents of the examples.in module.
    fs::write(src_dir.join("examples.in"), examples_in)?;

    // Write out the index.html file
    fs::write(crate_dir.join("index.html"), index_html)?;

    Ok(())
}
