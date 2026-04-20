use std::{collections::HashSet, sync::LazyLock};

use aho_corasick::AhoCorasick;

pub static TAGS: LazyLock<Vec<&'static str>> = LazyLock::new(|| {
    fn leak<T: ToString>(name: T) -> &'static str {
        Box::leak(name.to_string().into_boxed_str()) as &'static str
    }

    include_str!("./tags.txt")
        .lines()
        .filter(|l| !(l.starts_with('#') || l.is_empty()))
        .flat_map(|l| {
            if l.contains(' ') {
                vec![
                    //
                    leak(l.replace(" ", "-")),
                    leak(l.replace(" ", "_")),
                    leak(l.replace(" ", "")),
                    l,
                ]
            } else {
                vec![l]
            }
        })
        .collect()
});

static TAG_MATCHER: LazyLock<AhoCorasick> =
    LazyLock::new(|| AhoCorasick::builder().build(TAGS.iter()).unwrap());

pub fn tag_string(text: &str) -> HashSet<&'static str> {
    // TODO dedupicate and postprocess
    TAG_MATCHER
        .find_iter(&text.to_lowercase())
        .filter_map(|p| TAGS.get(p.pattern().as_usize()).copied())
        .collect()
}
