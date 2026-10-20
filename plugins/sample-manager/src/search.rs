//! Fuzzy search (fzf's way: the words of the query each match, in any order) over file names: the
//! scanned files and the plugins' caches.
use crate::scan::Sample;
use fuzzy_matcher::{FuzzyMatcher, skim::SkimMatcherV2};
use rayon::prelude::*;

/// The most results the table gets.
pub const MAX: usize = 100;

/// The indices into `files` of the best `MAX` matches of `query` on the file name, best first; with
/// no query, the first `MAX` files. A match has every space-separated word of the query in it.
pub fn search(files: &[Sample], query: &str) -> Vec<usize> {
    let words: Vec<&str> = query.split_whitespace().collect();
    if words.is_empty() {
        return (0..files.len().min(MAX)).collect();
    }
    let m = SkimMatcherV2::default();
    let mut hits: Vec<(i64, usize)> = files
        .par_iter()
        .enumerate()
        .filter_map(|(i, f)| {
            let score = words
                .iter()
                .try_fold(0, |sum, w| Some(sum + m.fuzzy_match(&f.name, w)?));
            score.map(|s| (s, i))
        })
        .collect();
    // a full sort, `select_nth_unstable` if a million files ever make it slow
    hits.sort_unstable_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    hits.into_iter().take(MAX).map(|(_, i)| i).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn samples(names: &[&str]) -> Vec<Sample> {
        names
            .iter()
            .map(|n| Sample {
                name: n.to_string(),
                path: n.into(),
                ..Default::default()
            })
            .collect()
    }

    #[test]
    fn matches_fuzzily_best_first_and_caps_the_list() {
        let f = samples(&["snare.wav", "kick_hard.wav", "big_kick.wav", "hat.wav"]);
        let r = search(&f, "kck");
        assert_eq!(r.len(), 2);
        assert!(r.contains(&1) && r.contains(&2));
        assert_eq!(f[search(&f, "snare")[0]].name, "snare.wav");
        assert!(search(&f, "zzz").is_empty());
        // words match in any order, each on its own; all of them must
        assert_eq!(
            search(
                &samples(&["Kick_Power-01.mp3", "Kick_Soft.mp3"]),
                "power kick"
            ),
            [0]
        );
        assert_eq!(search(&f, "  ").len(), 4);
        let many = samples(&vec!["a.wav"; 500]);
        assert_eq!(
            (search(&many, "").len(), search(&many, "a").len()),
            (MAX, MAX)
        );
    }
}
