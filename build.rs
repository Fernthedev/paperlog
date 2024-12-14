extern crate cbindgen;

use std::env;

fn main() {
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();

    cbindgen::Builder::new()
        .with_crate(crate_dir)
        .with_language(cbindgen::Language::C)
        .with_namespaces(&["Paper", "ffi"])
        .with_cpp_compat(true)
        .with_pragma_once(true)
        .with_item_prefix("paper2_")
        .generate()
        .expect("Unable to generate bindings")
        .write_to_file("./shared/bindings.h");
}
