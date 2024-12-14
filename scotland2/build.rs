use std::env;

fn main() -> color_eyre::Result<()> {
    if env::var("CARGO_CFG_TARGET_OS").unwrap() == "android" {
        use bindgen::builder;

        let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();

        // Configure and generate bindings.
        let bindings = builder()
            .header("./extern/includes/scotland2/shared/modloader.h")
            .clang_arg("-I./extern/includes/scotland2/shared")
            // .clang_arg("-Istdbool.h") // for some reason this is necessary
            .clang_arg("-include")
            .clang_arg("stdbool.h")
            .generate()?;

        // Write the generated bindings to an output file.
        let path = format!("{crate_dir}/src/scotland2.rs");
        bindings.write_to_file(path)?;
    }
    Ok(())
}
