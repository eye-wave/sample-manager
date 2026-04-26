use chumsky::{input::Stream, prelude::*};
use logos::Logos;
use std::{fs, path::Path};

use crate::{
    ast::{Trie, build_trie, emit_c},
    lexer::Token,
    parser::make_parser,
};

mod ast;
mod lexer;
mod parser;

fn print_size<T: AsRef<[u8]>>(input: T) {
    let bytes = input.as_ref().len() as f64;

    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];

    let mut size = bytes;
    let mut unit = 0;

    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }

    println!("{:.2} {}", size, UNITS[unit]);
}

fn main() {
    let src = include_str!("../../tags.tree");
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../src/gen/tags.h");

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

    for e in &errors {
        eprintln!("parse error: {e:?}");
    }

    let ast = match ast {
        Some(items) => items,
        None => {
            eprintln!("fatal: parse produced no output");
            std::process::exit(1);
        }
    };

    let mut trie = Trie::new();
    build_trie(&ast, &mut trie, &[]);

    let code = emit_c(&trie);
    print_size(&code);

    fs::create_dir(&path.parent().unwrap()).ok();
    fs::write(path, code).expect("Failed to write C header");
}
