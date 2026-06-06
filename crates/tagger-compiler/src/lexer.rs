use logos::Logos;

#[derive(Logos, Debug, PartialEq, Eq, Hash, Clone)]
#[logos(skip r"[ \t]+")]
pub enum Token {
    #[regex(r"//[^\n\r]*", logos::skip, allow_greedy = true)]
    Comment,

    #[regex(r"[a-zA-Z0-9]+", |lex| lex.slice().to_string())]
    Ident(String),

    #[token("!")]
    Bang,
    #[token("*")]
    Star,
    #[token("+")]
    Plus,
    #[token("#")]
    Hash,
    #[token("{")]
    LBrace,
    #[token("}")]
    RBrace,

    #[token("\n")]
    #[token("\r")]
    Newline,
}
