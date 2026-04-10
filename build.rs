use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    let input_dir = Path::new("styles");
    let output_dir = Path::new("client/css");

    println!("cargo:rerun-if-changed=styles");

    if !output_dir.exists() {
        fs::create_dir_all(output_dir).unwrap();
    }

    for entry in fs::read_dir(input_dir).unwrap() {
        let path = entry.unwrap().path();

        if !path.is_file() {
            continue;
        }

        match path.extension().and_then(|e| e.to_str()) {
            Some("scss") => compile_scss(&path, output_dir),
            Some("css") => copy_css(&path, output_dir),
            _ => {}
        }
    }
}

fn compile_scss(input: &Path, output_dir: &Path) {
    let css = grass::from_path(
        input,
        &grass::Options::default().style(grass::OutputStyle::Compressed),
    )
    .unwrap_or_else(|e| panic!("SCSS compile error in {:?}: {}", input, e));

    let file_name = input.file_stem().unwrap();
    let mut out_path = output_dir.to_path_buf();
    out_path.push(file_name);
    out_path.set_extension("css");

    fs::write(out_path, css).unwrap();
}

fn copy_css(input: &Path, output_dir: &Path) {
    let file_name = input.file_name().unwrap();
    let mut out_path = PathBuf::from(output_dir);
    out_path.push(file_name);

    fs::copy(input, out_path).unwrap();
}
