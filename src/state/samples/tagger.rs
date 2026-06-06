use tagger_runner::Model;

const TAGGER_MODEL: Model = Model::from_bytes(include_bytes!("../../../target/output/tagger.bin"));

pub fn tag_string(text: &str) -> Vec<&'static str> {
    let mut tags = Vec::new();

    TAGGER_MODEL.search(text, |tag| tags.push(tag));
    tags
}
