pub mod lexer;
pub mod parser;
pub mod trie;

use chumsky::{input::Stream, prelude::*};
use logos::Logos;

pub use crate::{lexer::Token, parser::make_parser, trie::*};

pub fn compile(src: &str) -> Result<Vec<u8>, CompileError> {
    let tokens: Vec<Token> = Token::lexer(src)
        .enumerate()
        .filter_map(|(i, r)| match r {
            Ok(t) => Some(t),
            Err(_) => {
                eprintln!("lex error at token {i}");
                None
            }
        })
        .collect();

    let stream = Stream::from_iter(tokens);
    let (ast, errors) = make_parser().parse(stream).into_output_errors();

    if !errors.is_empty() {
        return Err(CompileError::Parse(
            errors.iter().map(|e| format!("{e:?}")).collect(),
        ));
    }

    let ast = ast.ok_or(CompileError::NoOutput)?;
    let mut string_table = StringTable::new();
    let trie = build_dfa(&ast, &mut string_table);

    Ok(emit_binary(&trie, &string_table))
}

#[derive(Debug)]
pub enum CompileError {
    Parse(Vec<String>),
    NoOutput,
}
