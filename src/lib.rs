use chrono::{DateTime, Utc};
use chumsky::prelude::*;

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

#[derive(Clone, Debug, PartialEq, Eq)]
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

pub fn parse_type<'src>() -> impl Parser<'src, &'src str, Type> {
    choice((
        just("block").to(Type::Block),
        just("char").to(Type::Char),
        just("dir").to(Type::Dir),
        just("fifo").to(Type::Fifo),
        just("file").to(Type::File),
        just("link").to(Type::Link),
        just("socket").to(Type::Socket),
    ))
}

pub fn parse_timestamp<'src>() -> impl Parser<'src, &'src str, DateTime<Utc>> {
    // TODO: do we reeeally need to handle negatives?
    let number_i64 = text::int::<_, extra::Err<EmptyErr>>(10)
        .to_slice()
        .try_map(|s: &str, _| s.parse::<i64>().map_err(|_| EmptyErr::default()));

    let number_u32 = text::int::<_, extra::Err<EmptyErr>>(10)
        .to_slice()
        .try_map(|s: &str, _| s.parse::<u32>().map_err(|_| EmptyErr::default()));

    number_i64
        .then_ignore(just('.'))
        .then(number_u32)
        .try_map(|(secs, nsecs), _| {
            DateTime::from_timestamp(secs, nsecs).ok_or(EmptyErr::default())
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_type() {
        assert_eq!(parse_type().parse("block").into_result(), Ok(Type::Block));
        assert_eq!(parse_type().parse("char").into_result(), Ok(Type::Char));
        assert_eq!(parse_type().parse("dir").into_result(), Ok(Type::Dir));
        assert_eq!(parse_type().parse("fifo").into_result(), Ok(Type::Fifo));
        assert_eq!(parse_type().parse("file").into_result(), Ok(Type::File));
        assert_eq!(parse_type().parse("link").into_result(), Ok(Type::Link));
        assert_eq!(parse_type().parse("socket").into_result(), Ok(Type::Socket));
    }

    #[test]
    fn test_parse_timestamp() {
        assert_eq!(
            parse_timestamp().parse("1630456800.0").into_result(),
            Ok(DateTime::from_timestamp(1630456800, 0).unwrap())
        );
        assert_eq!(
            parse_timestamp()
                .parse("1769640177.434772208")
                .into_result(),
            Ok(DateTime::from_timestamp(1769640177, 434772208).unwrap())
        );
    }
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
