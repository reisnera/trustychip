use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=wrapper.h");

    let bindings = bindgen::Builder::default()
        .header("wrapper.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .allowlist_var(r"(?i)retro.*")
        .allowlist_type(r"(?i)retro.*")
        .default_enum_style(bindgen::EnumVariation::Rust {
            non_exhaustive: false,
        })
        .layout_tests(true)
        .generate()
        .expect("Unable to generate bindings");

    bindings
        .write_to_file(Path::new("./src/bindings.rs"))
        .expect("Couldn't write bindings");
}
