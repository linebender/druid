#[cfg(not(feature = "x11"))]
fn main() {}

#[cfg(feature = "x11")]
fn main() {
    use pkg_config::probe_library;
    use std::env;
    use std::path::PathBuf;

    if env::var("CARGO_CFG_TARGET_OS").unwrap() != "linux" {
        return;
    }

    probe_library("xkbcommon").unwrap();
    probe_library("xkbcommon-x11").unwrap();

    let bindings = bindgen::Builder::default()
        // The input header we would like to generate
        // bindings for.
        .header_contents(
            "wrapper.h",
            "\
#include <xkbcommon/xkbcommon-compose.h>
#include <xkbcommon/xkbcommon-names.h>
#include <xkbcommon/xkbcommon-x11.h>
#include <xkbcommon/xkbcommon.h>",
        )
        // Tell cargo to invalidate the built crate whenever any of the
        // included header files changed.
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .prepend_enum_name(false)
        .size_t_is_usize(true)
        .allowlist_function("xkb_.*")
        .allowlist_type("xkb_.*")
        .allowlist_var("XKB_.*")
        .allowlist_type("xcb_connection_t")
        // this needs var args
        .blocklist_function("xkb_context_set_log_fn")
        // we use FILE from libc
        .blocklist_type("FILE")
        .blocklist_type("va_list")
        .blocklist_type("_.*")
        .generate()
        .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/xkbcommon.rs file.
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("xkbcommon_sys.rs"))
        .expect("Couldn't write bindings!");
}
