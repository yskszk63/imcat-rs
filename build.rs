//use std::path::PathBuf;
//use std::env;

fn main() {
    let target = std::env::var("TARGET").unwrap();
    if target.contains("wasm32") {
        //https://users.rust-lang.org/t/how-to-static-link-c-lib-to-wasm/36558/5
        std::env::set_var("CC", "clang");
        std::env::set_var("AR", "llvm-ar");
        std::env::set_var("CFLAGS", "--sysroot /opt/wasi-sdk/wasi-sysroot");
    }

    println!("cargo:rerun-if-changed=src/bindings.c");
    cc::Build::new()
        .file("src/bindings.c")
        .include("src")
        .compile("bindings");

    /*
    println!("cargo:rerun-if-changed=src/stb_image.h");
    let bindings = bindgen::Builder::default()
        .header("src/stb_image.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .clang_arg("--sysroot=/opt/wasi-sdk/wasi-sysroot")
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
    */
}
