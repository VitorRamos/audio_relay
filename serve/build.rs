use std::path::PathBuf;

fn main() {
    println!("cargo:rustc-link-search=libs/");
    println!("cargo:rustc-link-lib=openaptx");

    let bindings = bindgen::Builder::default()
        .header("../openaptx.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .expect("Unable to generate bindings");

    if !std::process::Command::new("cc")
        .arg("-c")
        .arg("-o")
        .arg("libs/libopenaptx.o")
        .arg("-O3")
        .arg("-mavx2")
        .arg("-fPIC")
        .arg("../openaptx.c")
        .output()
        .expect("could not spawn `clang`")
        .status
        .success()
    {
        panic!("could not compile object file");
    }

    if !std::process::Command::new("ar")
        .arg("r")
        .arg("libs/libopenaptx.a")
        .arg("libs/libopenaptx.o")
        .output()
        .expect("could not spawn `clang`")
        .status
        .success()
    {
        panic!("could not compile object file");
    }

    let out_path = PathBuf::from("src/");
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}