use chrono::Utc;
use chumsky::{prelude::*, text::whitespace};

pub struct Entry {
    pub path: String,
}

pub enum Keyword {
    Type(Type),
    Uid(u32),
    Time(chrono::DateTime<Utc>),
    Size(u64),
    Sha256(String),
    Link(String),
}

pub enum Type {
    Block,
    Char,
    Dir,
    Fifo,
    File,
    Link,
    Socket,
}

pub enum Command {
    Set(Vec<Keyword>),
    Unset,
}

// fn parse_keyword<'a>() -> impl Parser<'a, &'a str, Keyword> {

// }

fn parse<'a>() -> impl Parser<'a, &'a str, Vec<Entry>> {
    let type_value = choice((
        just("block").map(|_| Type::Block),
        just("char").map(|_| Type::Char),
        just("dir").map(|_| Type::Dir),
        just("fifo").map(|_| Type::Fifo),
        just("file").map(|_| Type::File),
        just("link").map(|_| Type::Link),
        just("socket").map(|_| Type::Socket),
    ));

    let keyword = choice((
        just("type")
            .ignore_then(just("="))
            .ignore_then(type_value)
            .map(|r#type| Keyword::Type(r#type)),
        just("uid")
            .ignore_then(just("="))
            .ignore_then(text::digits(10).to_slice())
            .map(|uid| Keyword::Uid(uid)),
        just("time")
            .ignore_then(just("="))
            .ignore_then(text::digits(10).to_slice())
            .map(|time| Keyword::Time(time)),
        just("size")
            .ignore_then(just("="))
            .ignore_then(text::digits(10).to_slice())
            .map(|size| Keyword::Size(size)),
        just("sha256")
            .ignore_then(just("="))
            .ignore_then(text::digits(10).to_slice())
            .map(|sha256| Keyword::Sha256(sha256)),
        just("link")
            .ignore_then(just("="))
            .ignore_then(text::digits(10).to_slice())
            .map(|link| Keyword::Link(link)),
    ));

    let keywords = keyword.padded();

    let command = just('/').then(whitespace()).then(keywords);
}
