use std::path::PathBuf;

fn main() {
    #[allow(unused_mut)]
    let mut path = PathBuf::from("tagger").join("build");

    if cfg!(target_os = "windows") {
        path.push("Release");
    }

    println!("cargo:rustc-link-search=native={}", path.display());
    println!("cargo:rustc-link-lib=static=libtagger");
    println!("cargo:rerun-if-changed={}", path.display());
}
