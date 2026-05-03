use std::{path::PathBuf, sync::OnceLock};

pub const APP_NAME: &str = "SampleVault";
const PLUGIN_DIR: &str = "plug-ins";

macro_rules! define_paths {
    (
        $(
            fn $name:ident => $base:ident $(.join($segment:expr))*
        )*
    ) => {

        $(
            paste::paste! {
                static [< $name:upper >]: OnceLock<PathBuf> = OnceLock::new();

                pub fn $name() -> &'static PathBuf {
                    [< $name:upper >].get_or_init(|| {
                        let base = match stringify!($base) {
                            "cache" => dirs::cache_dir().unwrap().join(APP_NAME),
                            "config" => dirs::config_local_dir().unwrap().join(APP_NAME),
                            "data" => dirs::data_dir().unwrap().join(APP_NAME),
                            _ => unreachable!(),
                        };

                        base $(.join($segment))*
                    })
                }
            }
        )*

        pub fn create_all_dirs() -> std::io::Result<()> {
            $(
                std::fs::create_dir_all($name())?;
            )*
            Ok(())
        }
    };
}

define_paths! {
    fn config_file => config.join("config.toml")
    fn favorites_file => cache.join(".favorites")
    fn themes_path => config.join("themes")
    fn thumbnail_cache_path => cache.join(".waves")

    fn plugin_sync_path => data.join("Samples")
    fn plugin_cache_path => cache.join(PLUGIN_DIR)
    fn plugin_config_path => config.join(PLUGIN_DIR)
}
