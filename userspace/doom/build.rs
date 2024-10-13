use std::env;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ref dg_src_dir = std::path::PathBuf::from("doomgeneric/doomgeneric");
    let mut dg_c_paths = vec![];
    let mut dg_h_paths = vec![];

    // Find most c and h files
    for entry in std::fs::read_dir(dg_src_dir)? {
        let entry = entry?;
        if let Some(filename) = entry.file_name().to_str() {
            if filename.starts_with("doomgeneric_")
                || filename == "i_main.c"
                || filename.contains("sdl")
                || filename.contains("allegro")
            {
                continue;
            }

            if filename.ends_with(".h") {
                dg_h_paths.push(dg_src_dir.join(filename));
            } else if filename.ends_with(".c") {
                dg_c_paths.push(dg_src_dir.join(filename));
            }
        }
    }
    dg_c_paths
        .iter()
        .chain(dg_h_paths.iter())
        .for_each(|path| println!("cargo:rerun-if-changed={}", path.to_str().unwrap()));

    cc::Build::new()
        .flag("-w") // Disable warnings
        .files(dg_c_paths)
        .compile("doomgeneric");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());

    bindgen::Builder::default()
        .header("doomgeneric/doomgeneric/doomkeys.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .expect("Unable to generate bindings")
        .write_to_file(out_path.join("keys.rs"))
        .expect("Couldn't write bindings!");

    bindgen::Builder::default()
        .header("doomgeneric/doomgeneric/doomgeneric.h")
        .allowlist_file("doomgeneric/doomgeneric/doomgeneric.h")
        .blocklist_function("^DG_.*")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .expect("Unable to generate bindings")
        .write_to_file(out_path.join("doomgeneric.rs"))
        .expect("Couldn't write bindings!");

    println!("cargo:rustc-link-lib=static=doomgeneric");
    Ok(())
}
