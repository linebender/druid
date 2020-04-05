use std::io::Result;
use std::path::PathBuf;
use std::{env, fs};

fn main() -> Result<()> {
    let crate_dir = PathBuf::from(&env::var("CARGO_MANIFEST_DIR").unwrap());
    let examples_dir = crate_dir.join("src").join("examples");
    
    let parent_dir = crate_dir.parent().unwrap();

    // Create a symlink (platform specific) to the examples directory.
    #[cfg(unix)]
    std::os::unix::fs::symlink(parent_dir, &examples_dir).ok();
    #[cfg(windows)]
    std::os::windows::fs::symlink_dir(parent_dir, &examples_dir).ok();

    // Get a list of examples

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
        </html>
        "#,
                name = example_str
            );

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
    Ok(())
}
