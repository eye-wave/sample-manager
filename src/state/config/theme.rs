use std::{collections::HashMap, fs};

use serde::{Deserialize, Serialize};

use crate::{AStr, LogErrorExt, state::app_paths};

use super::color::{RGBAColor, RGBColor};

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, PartialOrd, Eq, Ord)]
#[serde(rename_all = "lowercase")]
pub enum ThemeType {
    Light,
    Dark,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Theme {
    pub theme_type: ThemeType,
    bg_base: RGBColor,
    bg_panel: RGBColor,
    bg_surface: RGBColor,
    bg_hover: RGBColor,
    bg_active: RGBColor,
    accent: RGBColor,
    accent_dim: RGBColor,
    accent_glow: RGBAColor,
    text_primary: RGBColor,
    text_secondary: RGBColor,
    text_muted: RGBColor,
    border: RGBAColor,
    border_active: RGBAColor,
    tag_bg: RGBColor,
    tag_text: RGBColor,
    tag_text_secondary: RGBColor,
    love: RGBColor,
    wave_a: RGBColor,
    wave_b: RGBColor,
    radius: u8,
}

impl Default for Theme {
    fn default() -> Self {
        toml::from_str(include_str!("./themes/valentine.toml")).unwrap()
    }
}

impl Theme {
    pub fn to_css(&self) -> String {
        let Theme {
            theme_type: _,
            bg_base,
            bg_panel,
            bg_surface,
            bg_hover,
            bg_active,
            accent,
            accent_dim,
            accent_glow,
            text_primary,
            text_secondary,
            text_muted,
            border,
            border_active,
            tag_bg,
            tag_text,
            tag_text_secondary,
            love,
            wave_a,
            wave_b,
            radius,
        } = self;

        format!(
            ":root{{--bg-base:{bg_base};--bg-panel:{bg_panel};--bg-surface:{bg_surface};--bg-hover:{bg_hover};--bg-active:{bg_active};--accent:{accent};--accent-dim:{accent_dim};--accent-glow:{accent_glow};--text-primary:{text_primary};--text-secondary:{text_secondary};--text-muted:{text_muted};--border:{border};--border-active:{border_active};--tag-bg:{tag_bg};--tag-text:{tag_text};--tag-text-secondary:{tag_text_secondary};--love:{love};--wave-a:{wave_a};--wave-b:{wave_b};--radius:{radius}px}}"
        )
    }
}

pub fn list_themes() -> std::io::Result<HashMap<AStr, Vec<AStr>>> {
    let files = fs::read_dir(app_paths::themes_path())?
        .filter_map(Result::ok)
        .filter_map(|f| {
            let path = app_paths::themes_path().join(f.path());
            let theme: Theme =
                toml::from_str(&fs::read_to_string(&path).sure("Failed to read theme file")?)
                    .sure("Failed to parse theme")?;

            Some((
                theme.theme_type,
                f.file_name()
                    .to_string_lossy()
                    .into_owned()
                    .replace(".toml", ""),
            ))
        })
        .collect::<Vec<_>>();

    let collect_themes = |t: ThemeType| {
        let mut themes: Vec<_> = files
            .iter()
            .filter(|f| f.0 == t)
            .map(|f| f.1.as_str().into())
            .collect();

        themes.sort();
        themes
    };

    Ok(HashMap::from([
        ("light".into(), collect_themes(ThemeType::Light)),
        ("dark".into(), collect_themes(ThemeType::Dark)),
    ]))
}
