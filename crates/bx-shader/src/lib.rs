#[macro_export]
macro_rules! include_shader {
    ($name:literal) => {
        include_bytes!(concat!(env!("OUT_DIR"), "/", $name, ".spv"))
    };
}

#[cfg(feature = "build")]
pub fn main(path: impl AsRef<std::path::Path>) {
    use std::path::Path;
    use std::{env, fs};

    use naga::back::spv;
    use naga::valid::{Capabilities, ValidationFlags, Validator};

    println!("cargo:rerun-if-changed=shaders");
    let out = env::var("OUT_DIR").unwrap();

    for entry in fs::read_dir(path.as_ref()).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("wgsl") {
            continue;
        }
        let name = path.file_name().unwrap().to_str().unwrap();
        let src = fs::read_to_string(&path).unwrap();

        let module = naga::front::wgsl::parse_str(&src)
            .unwrap_or_else(|e| panic!("{}/{name}: {}", path.display(), e.emit_to_string(&src)));
        let info = Validator::new(ValidationFlags::all(), Capabilities::all())
            .validate(&module)
            .unwrap_or_else(|e| panic!("{}/{name}: validation error:\n{e:?}", path.display()));

        let options = spv::Options {
            flags: spv::WriterFlags::empty(),
            ..Default::default()
        };
        let words = spv::write_vec(&module, &info, &options, None)
            .unwrap_or_else(|e| panic!("shaders/{name}: SPIR-V error:\n{e:?}"));

        let bytes: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
        let spv_path = Path::new(&out).join(format!("{name}.spv"));
        fs::write(&spv_path, &bytes).unwrap();

        if env::var("PROFILE").as_deref() == Ok("release") {
            match std::process::Command::new("spirv-opt")
                .arg("-O")
                .arg(&spv_path)
                .arg("-o")
                .arg(&spv_path)
                .status()
            {
                Ok(status) if status.success() => {}
                Ok(status) => {
                    println!(
                        "cargo:warning=spirv-opt exited with status {status} for shader {name}"
                    );
                }
                Err(e) => {
                    println!("cargo:warning=failed to spawn spirv-opt for shader {name}: {e}");
                }
            }
        }
    }
}
