use std::path::PathBuf;
use std::env;

fn main() {
    println!("cargo:rerun-if-changed=src/bindings.c");
    cc::Build::new()
        .file("src/bindings.c")
        .include("src")
        .compile("bindings");


    println!("cargo:rerun-if-changed=src/stb_image.h");
    let bindings = bindgen::Builder::default()
        .header("src/stb_image.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
