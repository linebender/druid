use std::io::Result;
use std::path::PathBuf;
use std::{env, fs};

/// Examples known to not work with WASM are skipped. Ideally this list will eventually be empty.
const EXCEPTIONS: &[&'static str] = &[
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

    // Declare the newly found example modules in examples.rs
    let mut example_rs = r#"
//! This is an automatically generated module collecting all valid examples
//! in the parent examples directory.

// This file must not be committed unless it contains no modules.
// Rustfmt will fail if this file is not empty in the repository --- it does not run the
// "build.rs" script, and the examples module directory symlink is created at build time in
// "build.rs".

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

            // Skip examples that are known to not work.
            if EXCEPTIONS.contains(&example_str.as_ref()) {
                continue;
            }

            // Record the valid example module we found to add to the generated examples.rs
            example_rs.push_str(&format!("pub mod {};\n", example_str));

            // Create an html document for each example.
            let html = format!(
                r#"
<!DOCTYPE html>
<html lang="en">
    <head>
        <meta charset="utf-8">
        <title>Druid WASM example - {name}</title>
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
                name = example_str
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

    // Write out the contents of the examples.rs module.
    //fs::write(src_dir.join("examples.rs"), example_rs)?;

    Ok(())
}
