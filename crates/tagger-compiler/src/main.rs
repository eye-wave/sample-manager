use chumsky::{input::Stream, prelude::*};
use clap::Parser as ClapParser;
use logos::Logos;
use std::{fs, path::PathBuf};

mod lexer;
mod parser;
mod trie;

use crate::{
    lexer::Token,
    parser::make_parser,
    trie::{StringTable, build_dfa, emit_binary},
};

#[derive(ClapParser, Debug)]
#[command(name = "trie-compile", version, about)]
struct Args {
    input: PathBuf,
    #[arg(short, long, default_value = "model.bin")]
    output: PathBuf,
    #[arg(short, long)]
    verbose: bool,
    #[arg(short, long, default_value = "false")]
    debug: bool,
}

fn main() {
    let args = Args::parse();

    let src = fs::read_to_string(&args.input).unwrap_or_else(|e| {
        eprintln!("error: could not read {:?}: {e}", args.input);
        std::process::exit(1);
    });

    let tokens: Vec<Token> = Token::lexer(&src)
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

    let mut st = StringTable::new();
    let dfa = build_dfa(&ast, &mut st);
    let binary = emit_binary(&dfa, &st);

    if let Some(parent) = args.output.parent() {
        fs::create_dir_all(parent).ok();
    }
    fs::write(&args.output, &binary).unwrap_or_else(|e| {
        eprintln!("error: could not write {:?}: {e}", args.output);
        std::process::exit(1);
    });

    println!("wrote {:?}", args.output);
    print_size(binary.len());

    if args.debug {
        dfa.log_stats();
        dfa.log_offset_stats();
    }
}

fn print_size(bytes: usize) {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit = 0;
    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    println!("total: {size:.2} {}", UNITS[unit]);
}
