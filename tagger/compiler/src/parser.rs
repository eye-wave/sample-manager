use chumsky::{input::Stream, prelude::*, recursive::Indirect};

use crate::lexer::Token;

#[derive(Debug, Clone, PartialEq)]
pub struct PatChar {
    pub ch: char,
    pub repeating: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Output {
    Itself,
    Named(String),
}

#[derive(Debug, Clone)]
pub struct WordDecl {
    pub pattern: Vec<PatChar>,
    pub outputs: Vec<Output>,
}

#[derive(Debug, Clone)]
pub enum Item {
    Word(WordDecl),
    Group { name: String, items: Vec<Item> },
}

type Inp<'src> = Stream<std::vec::IntoIter<Token>>;
type Err<'src> = Rich<'src, Token>;
type Extra<'src> = extra::Err<Err<'src>>;

pub fn make_parser<'src>() -> impl Parser<'src, Inp<'src>, Vec<Item>, Extra<'src>> {
    let nl = just::<Token, Inp<'src>, Extra<'src>>(Token::Newline);
    let nls = nl.clone().repeated().ignored();

    let ident = select! { Token::Ident(s) => s };

    let hash_piece = just(Token::Hash).map(|_| {
        vec![PatChar {
            ch: '_',
            repeating: false,
        }]
    });

    let pat_piece = ident
        .then(just(Token::Plus).or_not())
        .map(|(s, plus): (String, _)| {
            let chars: Vec<char> = s.chars().collect();
            let last = chars.len().saturating_sub(1);
            chars
                .into_iter()
                .enumerate()
                .map(|(i, ch)| PatChar {
                    ch,
                    repeating: i == last && plus.is_some(),
                })
                .collect::<Vec<_>>()
        });

    let pattern = hash_piece
        .clone()
        .or(pat_piece)
        .repeated()
        .at_least(1)
        .collect::<Vec<Vec<PatChar>>>()
        .map(|vv| vv.into_iter().flatten().collect::<Vec<PatChar>>());

    let output_clause =
        just(Token::Star)
            .ignore_then(ident.clone().or_not())
            .map(|opt| match opt {
                Some(name) => Output::Named(name),
                None => Output::Itself,
            });

    let word_decl = pattern
        .then(output_clause.repeated().collect::<Vec<_>>())
        .map(|(pattern, outputs)| WordDecl { pattern, outputs });

    let mut item: Recursive<Indirect<'src, '_, Inp<'src>, Item, Extra<'src>>> =
        Recursive::declare();

    let group = ident
        .clone()
        .then_ignore(nls.clone())
        .then(item.clone().repeated().collect::<Vec<_>>().delimited_by(
            just(Token::LBrace).then(nls.clone()),
            nls.clone().then(just(Token::RBrace)),
        ))
        .then_ignore(nls.clone())
        .map(|(name, items)| Item::Group { name, items });

    let word_item = word_decl
        .then_ignore(
            nl.clone()
                .repeated()
                .at_least(1)
                .ignored()
                .or(end().rewind()),
        )
        .map(Item::Word);

    item.define(group.or(word_item));

    nls.clone()
        .ignore_then(item.repeated().collect::<Vec<_>>())
        .then_ignore(nls)
        .then_ignore(end())
}
