//! Helpers for search plugins (see `wit/search.wit`).

/// `icon!("icon.rgba", 32)`: the plugin's icon, embedded as raw pixels. The file (next to the
/// source file that calls this) is `32 * 32 * 4` bytes of RGBA, rows top to bottom, no image
/// format; the one size is both sides, so the icon is always square. A size outside 32..=128 or a
/// file of the wrong length does not compile. Expands to the `Icon` of the `wit_bindgen::generate!`
/// of the calling crate, which has to be in scope.
///
/// ```ignore
/// fn describe() -> Info {
///     Info { name: "Mine".into(), description: "...".into(), icon: icon!("icon.rgba", 32), hosts: vec![] }
/// }
/// ```
#[macro_export]
macro_rules! icon {
    ($path:literal, $size:literal) => {{
        const RGBA: &[u8] = include_bytes!($path);
        const _: () = assert!($size >= 32 && $size <= 128, "an icon is 32 to 128 px");
        const _: () = assert!(
            RGBA.len() == $size * $size * 4,
            "the icon file is not size * size * 4 bytes of RGBA"
        );
        Icon {
            width: $size,
            height: $size,
            rgba: RGBA.to_vec(),
        }
    }};
}
