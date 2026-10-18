//! What survives a restart: the tracked directories, one path per line in
//! `<config dir>/sample-manager/dirs.txt`. Standalone and plugin share it.
use std::collections::HashSet;
use std::path::PathBuf;

fn file() -> Option<PathBuf> {
    Some(dirs::config_dir()?.join("sample-manager").join("dirs.txt"))
}

/// The tracked directories; the first run (no file yet) tracks `$HOME/Music` and saves that.
// a path with a newline in it does not round-trip; switch to postcard/serde if state grows
pub fn tracked_dirs() -> Vec<PathBuf> {
    let Some(f) = file() else { return vec![] };
    if let Ok(s) = std::fs::read_to_string(&f) {
        return s
            .lines()
            .filter(|l| !l.is_empty())
            .map(PathBuf::from)
            .collect();
    }
    let list: Vec<PathBuf> = dirs::home_dir()
        .map(|h| h.join("Music"))
        .into_iter()
        .collect();
    save(&f, &list);
    list
}

/// Starts tracking `p` (once) from the next start on.
pub fn add_dir(p: &std::path::Path) {
    let mut list = tracked_dirs();
    if !list.iter().any(|d| d == p)
        && let Some(f) = file()
    {
        list.push(p.to_path_buf());
        save(&f, &list);
    }
}

fn save(f: &std::path::Path, list: &[PathBuf]) {
    let text: Vec<_> = list.iter().map(|p| p.to_string_lossy()).collect();
    // not being able to save is not worth stopping for: the next start tracks the default again
    let _ = f.parent().map(std::fs::create_dir_all);
    let _ = std::fs::write(f, text.join("\n"));
}

/// Stops tracking `p` from the next start on.
pub fn remove_dir(p: &std::path::Path) {
    let mut list = tracked_dirs();
    if let Some(f) = file() {
        list.retain(|d| d != p);
        save(&f, &list);
    }
}

/// What the settings dialog changes, in `<config dir>/sample-manager/settings.txt`.
#[derive(Clone, Debug, PartialEq)]
pub struct Settings {
    /// An index of `theme::SCHEMES`.
    pub scheme: usize,
    /// The UI scale in percent.
    pub scale: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            scheme: 0,
            scale: 100,
        }
    }
}

/// The scales the dialog offers, in percent.
pub const SCALES: [u32; 4] = [80, 100, 125, 150];

impl Settings {
    /// `scheme=1` and `scale=125` lines; anything else (or garbage) keeps the default.
    fn parse(text: &str) -> Self {
        let mut s = Self::default();
        for (k, v) in text.lines().filter_map(|l| l.split_once('=')) {
            match (k.trim(), v.trim().parse::<u32>()) {
                ("scheme", Ok(v)) => s.scheme = (v as usize).min(crate::theme::SCHEMES.len() - 1),
                ("scale", Ok(v)) => s.scale = v.clamp(50, 200),
                _ => {}
            }
        }
        s
    }

    fn text(&self) -> String {
        format!("scheme={}\nscale={}\n", self.scheme, self.scale)
    }
}

fn settings_file() -> Option<PathBuf> {
    Some(
        dirs::config_dir()?
            .join("sample-manager")
            .join("settings.txt"),
    )
}

pub fn settings() -> Settings {
    settings_file()
        .and_then(|f| std::fs::read_to_string(f).ok())
        .map_or_else(Settings::default, |t| Settings::parse(&t))
}

pub fn save_settings(s: &Settings) {
    if let Some(f) = settings_file() {
        let _ = f.parent().map(std::fs::create_dir_all);
        let _ = std::fs::write(f, s.text());
    }
}

/// The favorite files, one path per line in `<cache dir>/sample-manager/favorites.txt`.
fn favorites_file() -> Option<PathBuf> {
    Some(
        dirs::cache_dir()?
            .join("sample-manager")
            .join("favorites.txt"),
    )
}

fn paths_from(text: &str) -> HashSet<PathBuf> {
    text.lines()
        .filter(|l| !l.is_empty())
        .map(PathBuf::from)
        .collect()
}

fn paths_text(set: &HashSet<PathBuf>) -> String {
    let mut v: Vec<_> = set.iter().map(|p| p.to_string_lossy()).collect();
    v.sort(); // the same set, the same file
    v.join("\n")
}

pub fn favorites() -> HashSet<PathBuf> {
    favorites_file()
        .and_then(|f| std::fs::read_to_string(f).ok())
        .map_or_else(HashSet::new, |t| paths_from(&t))
}

pub fn save_favorites(set: &HashSet<PathBuf>) {
    if let Some(f) = favorites_file() {
        let _ = f.parent().map(std::fs::create_dir_all);
        let _ = std::fs::write(f, paths_text(set));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn favorites_round_trip_sorted() {
        let set: HashSet<PathBuf> = ["/b/2.wav", "/a/1.wav"].map(PathBuf::from).into();
        assert_eq!(paths_text(&set), "/a/1.wav\n/b/2.wav");
        assert_eq!(paths_from(&paths_text(&set)), set);
        assert!(paths_from("").is_empty());
    }

    #[test]
    fn settings_round_trip_and_garbage_stays_in_range() {
        let s = Settings {
            scheme: 3,
            scale: 125,
        };
        assert_eq!(Settings::parse(&s.text()), s);
        let g = Settings::parse("scheme=99\nscale=100000\nnonsense\nscale=x");
        assert_eq!((g.scheme, g.scale), (crate::theme::SCHEMES.len() - 1, 200));
        assert_eq!(Settings::parse(""), Settings::default());
    }
}
