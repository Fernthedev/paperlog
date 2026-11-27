use std::{env, path::PathBuf};

fn main() {
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("android") {
        return;
    }

    cdylib_link_lines::metabuild();

    let extern_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("../extern");
    let header = extern_dir.join("includes/scotland2/modloader.h");
    let libs = extern_dir.join("libs");
    let library = libs.join("libsl2.so");
    assert!(
        header.is_file() && library.is_file(),
        "Run qpm2 restore from the Paper2 root to install Scotland2"
    );

    println!("cargo:rerun-if-env-changed=CARGO_NDK_SYSROOT_PATH");
    let sysroot = env::var("CARGO_NDK_SYSROOT_PATH").expect("Build Paper2 with cargo ndk");
    let bindings = bindgen::Builder::default()
        .header(header.to_str().unwrap())
        .clang_arg(format!("--sysroot={sysroot}"))
        .clang_args(["-include", "stdbool.h", "-include", "stddef.h"])
        .allowlist_function("modloader_get_application_id")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .expect("Unable to generate Scotland2 bindings");
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_dir.join("scotland2.rs"))
        .expect("Unable to write Scotland2 bindings");

    println!("cargo:rerun-if-changed={}", library.display());
    println!("cargo:rustc-link-search=native={}", libs.display());
    println!("cargo:rustc-link-lib=dylib=sl2");
}
