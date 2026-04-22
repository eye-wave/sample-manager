use std::path::PathBuf;

fn main() {
    let path = PathBuf::from("tagger").join("build");

    println!("cargo:rustc-link-search=native={}", path.display());
    println!("cargo:rustc-link-lib=static=libtagger");
}
