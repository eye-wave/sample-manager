use std::fmt::Display;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RGBColor(#[serde(with = "rgb_hex")] pub [u8; 3]);

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RGBAColor(#[serde(with = "rgba_hex")] pub [u8; 4]);

impl Display for RGBAColor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{}", hex_color::to_hex_rgba(&self.0))
    }
}

impl Display for RGBColor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{}", hex_color::to_hex_rgb(&self.0))
    }
}

mod hex_color {
    use serde::de::Error;

    fn expand_nibble(c: char) -> Result<u8, String> {
        let v =
            u8::from_str_radix(&c.to_string(), 16).map_err(|_| "invalid hex digit".to_string())?;
        Ok((v << 4) | v)
    }

    fn parse_pair(s: &str) -> Result<u8, String> {
        u8::from_str_radix(s, 16).map_err(|_| "invalid hex byte".to_string())
    }

    pub fn parse_rgba<E: Error>(input: &str) -> Result<[u8; 4], E> {
        let s = input.strip_prefix('#').unwrap_or(input);

        match s.len() {
            3 => {
                let r = expand_nibble(s.chars().next().unwrap()).map_err(E::custom)?;
                let g = expand_nibble(s.chars().nth(1).unwrap()).map_err(E::custom)?;
                let b = expand_nibble(s.chars().nth(2).unwrap()).map_err(E::custom)?;
                Ok([r, g, b, 255])
            }

            4 => {
                let r = expand_nibble(s.chars().next().unwrap()).map_err(E::custom)?;
                let g = expand_nibble(s.chars().nth(1).unwrap()).map_err(E::custom)?;
                let b = expand_nibble(s.chars().nth(2).unwrap()).map_err(E::custom)?;
                let a = expand_nibble(s.chars().nth(3).unwrap()).map_err(E::custom)?;
                Ok([r, g, b, a])
            }

            6 => Ok([
                parse_pair(&s[0..2]).map_err(E::custom)?,
                parse_pair(&s[2..4]).map_err(E::custom)?,
                parse_pair(&s[4..6]).map_err(E::custom)?,
                255,
            ]),

            8 => Ok([
                parse_pair(&s[0..2]).map_err(E::custom)?,
                parse_pair(&s[2..4]).map_err(E::custom)?,
                parse_pair(&s[4..6]).map_err(E::custom)?,
                parse_pair(&s[6..8]).map_err(E::custom)?,
            ]),

            _ => Err(E::custom("expected 3, 4, 6, or 8 hex chars")),
        }
    }

    pub fn to_hex_rgb(v: &[u8; 3]) -> String {
        format!("{:02x}{:02x}{:02x}", v[0], v[1], v[2])
    }

    pub fn to_hex_rgba(v: &[u8; 4]) -> String {
        format!("{:02x}{:02x}{:02x}{:02x}", v[0], v[1], v[2], v[3])
    }

    pub fn rgba_to_rgb(rgba: [u8; 4]) -> [u8; 3] {
        [rgba[0], rgba[1], rgba[2]]
    }
}

mod rgb_hex {
    use super::hex_color;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(value: &[u8; 3], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let rgb = [value[0], value[1], value[2]];
        serializer.serialize_str(&hex_color::to_hex_rgb(&rgb))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<[u8; 3], D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let rgba = hex_color::parse_rgba(&s)?;
        Ok(hex_color::rgba_to_rgb(rgba))
    }
}

mod rgba_hex {
    use super::hex_color;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(value: &[u8; 4], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&hex_color::to_hex_rgba(value))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<[u8; 4], D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        hex_color::parse_rgba(&s)
    }
}
