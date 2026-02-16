use std::path::PathBuf;

use chrono::{DateTime, Utc};
use chumsky::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub path: PathBuf,
    pub keywords: Vec<Keyword>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Keyword {
    Type(Type),
    Uid(u32),
    Time(chrono::DateTime<Utc>),
    Size(u64),
    Sha256(String),
    Link(PathBuf),
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

#[derive(Clone, Debug, PartialEq, Eq)]
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

pub fn parse_path<'src>() -> impl Parser<'src, &'src str, PathBuf> {
    none_of(" \t") // <-- NOTE: this will backfire
        .repeated()
        .at_least(1)
        .to_slice()
        .validate(|x: &str, _, _| PathBuf::from(x))
}

fn keyword_parser<'src, V>(
    key: impl Parser<'src, &'src str, &'src str>,
    value: impl Parser<'src, &'src str, V>,
) -> impl Parser<'src, &'src str, V> {
    key.ignore_then(just('=')).ignore_then(value)
}

pub fn parse_keyword<'src>() -> impl Parser<'src, &'src str, Keyword> {
    let type_value = parse_type();

    let number_u32 = text::int::<_, extra::Err<EmptyErr>>(10)
        .to_slice()
        .try_map(|s: &str, _| s.parse::<u32>().map_err(|_| EmptyErr::default()));

    let number_u64 = text::int::<_, extra::Err<EmptyErr>>(10)
        .to_slice()
        .try_map(|s: &str, _| s.parse::<u64>().map_err(|_| EmptyErr::default()));

    let timestamp = parse_timestamp();

    let path = parse_path();

    let sha256 = one_of("0123456789abcdefABCDEF")
        .repeated()
        .at_least(1)
        .to_slice();

    choice((
        keyword_parser(just("type"), type_value).map(Keyword::Type),
        keyword_parser(just("uid"), number_u32).map(Keyword::Uid),
        keyword_parser(just("time"), timestamp).map(Keyword::Time),
        keyword_parser(just("size"), number_u64).map(Keyword::Size),
        keyword_parser(choice((just("sha256digest"), just("sha256"))), sha256)
            .map(|hash: &str| Keyword::Sha256(hash.to_string())),
        keyword_parser(just("link"), path).map(|path: PathBuf| Keyword::Link(path)),
    ))
}

pub fn whitespace_with_continuation<'src>() -> impl Parser<'src, &'src str, ()> {
    choice((
        text::whitespace().map(|_| ()),
        just('\\')
            .then(text::newline())
            .then(text::whitespace().or_not())
            .map(|_| ()),
    ))
}

pub fn parse_keywords<'src>() -> impl Parser<'src, &'src str, Vec<Keyword>> {
    parse_keyword()
        .separated_by(whitespace_with_continuation())
        .collect()
}

pub fn parse_command<'src>() -> impl Parser<'src, &'src str, Command> {
    let unset = just("unset").to(Command::Unset);
    let set = just("set")
        .ignore_then(whitespace_with_continuation())
        .ignore_then(parse_keywords())
        .map(Command::Set);

    just('/')
        .ignore_then(choice((unset, set)))
        .then_ignore(end()) // <- not sure if this is needed, it may even break stuff
}

pub fn parse_entry<'src>() -> impl Parser<'src, &'src str, Entry> {
    let path = parse_path();
    let keywords = parse_keywords();

    path.padded_by(whitespace_with_continuation())
        .then(keywords)
        .map(|(path, keywords)| Entry { path, keywords })
}

pub fn parse_comment<'src>() -> impl Parser<'src, &'src str, ()> {
    just('#')
        .ignore_then(any().repeated())
        .ignore_then(choice((text::newline().ignore_then(end()), end()))) // <-- this may or may not be totally useles
        .ignored()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_type() {
        let parse = |input| parse_type().parse(input).into_result();

        assert_eq!(parse("block"), Ok(Type::Block));
        assert_eq!(parse("char"), Ok(Type::Char));
        assert_eq!(parse("dir"), Ok(Type::Dir));
        assert_eq!(parse("fifo"), Ok(Type::Fifo));
        assert_eq!(parse("file"), Ok(Type::File));
        assert_eq!(parse("link"), Ok(Type::Link));
        assert_eq!(parse("socket"), Ok(Type::Socket));
    }

    #[test]
    fn test_parse_timestamp() {
        let parse = |input| parse_timestamp().parse(input).into_result();

        assert_eq!(
            parse("1630456800.0"),
            Ok(DateTime::from_timestamp(1630456800, 0).unwrap())
        );
        assert_eq!(
            parse("1769640177.434772208"),
            Ok(DateTime::from_timestamp(1769640177, 434772208).unwrap())
        );
    }

    #[test]
    fn test_parse_path() {
        let parse = |input| parse_path().parse(input).into_result();

        assert_eq!(parse("foo.bar"), Ok(PathBuf::from("foo.bar")));
        assert_eq!(parse("../../foo.bar"), Ok(PathBuf::from("../../foo.bar")));
    }

    #[test]
    fn test_parse_type_keyword() {
        let parse = |input| parse_keyword().parse(input).into_result();

        assert_eq!(parse("type=block"), Ok(Keyword::Type(Type::Block)));
        assert_eq!(parse("type=char"), Ok(Keyword::Type(Type::Char)));
        assert_eq!(parse("type=dir"), Ok(Keyword::Type(Type::Dir)));
        assert_eq!(parse("type=fifo"), Ok(Keyword::Type(Type::Fifo)));
        assert_eq!(parse("type=file"), Ok(Keyword::Type(Type::File)));
        assert_eq!(parse("type=link"), Ok(Keyword::Type(Type::Link)));
        assert_eq!(parse("type=socket"), Ok(Keyword::Type(Type::Socket)));
    }

    #[test]
    fn test_parse_uid_keyword() {
        let parse = |input| parse_keyword().parse(input).into_result();

        assert_eq!(parse("uid=0"), Ok(Keyword::Uid(0)));
        assert_eq!(parse("uid=100"), Ok(Keyword::Uid(100)));
        assert_eq!(parse("uid=123456789"), Ok(Keyword::Uid(123456789)));
    }

    #[test]
    fn test_parse_timestamp_keyword() {
        let parse = |input| parse_keyword().parse(input).into_result();

        assert_eq!(
            parse("time=1630456800.0"),
            Ok(Keyword::Time(
                DateTime::from_timestamp(1630456800, 0).unwrap()
            ))
        );
        assert_eq!(
            parse("time=1769640177.434772208"),
            Ok(Keyword::Time(
                DateTime::from_timestamp(1769640177, 434772208).unwrap()
            ))
        );
    }

    #[test]
    fn test_parse_size_keyword() {
        let parse = |input| parse_keyword().parse(input).into_result();

        assert_eq!(parse("size=0"), Ok(Keyword::Size(0)));
        assert_eq!(parse("size=1024"), Ok(Keyword::Size(1024)));
        assert_eq!(parse("size=1048576"), Ok(Keyword::Size(1048576)));
    }

    #[test]
    fn test_parse_sha256_keyword() {
        let parse = |input| parse_keyword().parse(input).into_result();

        assert_eq!(
            parse("sha256=fd9849d9364b9b9aabed88a8aa8e007d7450c3ad3a17aee0617dd24959464249"),
            Ok(Keyword::Sha256(
                "fd9849d9364b9b9aabed88a8aa8e007d7450c3ad3a17aee0617dd24959464249".to_string()
            ))
        );
        assert_eq!(
            parse("sha256digest=fd9849d9364b9b9aabed88a8aa8e007d7450c3ad3a17aee0617dd24959464249"),
            Ok(Keyword::Sha256(
                "fd9849d9364b9b9aabed88a8aa8e007d7450c3ad3a17aee0617dd24959464249".to_string()
            ))
        );
    }

    #[test]
    fn test_parse_link_keyword() {
        let parse = |input| parse_keyword().parse(input).into_result();

        assert_eq!(
            parse("link=../../foo.bar"),
            Ok(Keyword::Link(PathBuf::from("../../foo.bar")))
        );
        assert_eq!(
            parse("link=./foo.bar"),
            Ok(Keyword::Link(PathBuf::from("./foo.bar")))
        );
        assert_eq!(
            parse("link=foo.bar"),
            Ok(Keyword::Link(PathBuf::from("foo.bar")))
        );
    }

    #[test]
    fn test_parse_keywords() {
        let parse = |input| parse_keywords().parse(input).into_result();

        assert_eq!(parse(""), Ok(vec![]));

        assert_eq!(parse("type=dir"), Ok(vec![Keyword::Type(Type::Dir),]));

        assert_eq!(
            parse("type=dir size=384 time=1769640373.412526597"),
            Ok(vec![
                Keyword::Type(Type::Dir),
                Keyword::Size(384),
                Keyword::Time(DateTime::from_timestamp(1769640373, 412526597).unwrap())
            ])
        );

        assert_eq!(
            parse("type=link size=24 time=1769203307.589764008"),
            Ok(vec![
                Keyword::Type(Type::Link),
                Keyword::Size(24),
                Keyword::Time(DateTime::from_timestamp(1769203307, 589764008).unwrap())
            ])
        );

        assert_eq!(
            parse(
                "size=10931 time=1769203027.452198079 \
                                sha256digest=014bb31e83d5c2e76aea1cc6e82217346ab41362f32cb355ad0f5c10aa0aeaff"
            ),
            Ok(vec![
                Keyword::Size(10931),
                Keyword::Time(DateTime::from_timestamp(1769203027, 452198079).unwrap()),
                Keyword::Sha256(
                    "014bb31e83d5c2e76aea1cc6e82217346ab41362f32cb355ad0f5c10aa0aeaff".to_string()
                )
            ])
        );
    }

    #[test]
    fn test_parse_commands() {
        let parse = |input| parse_command().parse(input).into_result();

        assert_eq!(
            parse("/set type=dir size=384 time=1769640373.412526597"),
            Ok(Command::Set(vec![
                Keyword::Type(Type::Dir),
                Keyword::Size(384),
                Keyword::Time(DateTime::from_timestamp(1769640373, 412526597).unwrap())
            ]))
        );

        assert_eq!(parse("/unset"), Ok(Command::Unset));
    }

    #[test]
    fn test_parse_comment() {
        let parse = |input| parse_comment().parse(input).into_result();

        assert_eq!(parse("#hello world"), Ok(()));
        assert_eq!(parse("# hello world"), Ok(()));
        assert_eq!(parse("#"), Ok(()));
    }

    #[test]
    fn test_parse_entry() {
        let parse = |input| parse_entry().parse(input).into_result();

        assert_eq!(
            parse("    LICENSE     size=10931 time=1769203027.452198079"),
            Ok(Entry {
                path: PathBuf::from("LICENSE"),
                keywords: vec![
                    Keyword::Size(10931),
                    Keyword::Time(DateTime::from_timestamp(1769203027, 452198079).unwrap())
                ]
            })
        );

        assert_eq!(
            parse(
                "    LICENSE     size=10931 time=1769203027.452198079 \
                            sha256digest=014bb31e83d5c2e76aea1cc6e82217346ab41362f32cb355ad0f5c10aa0aeaff"
            ),
            Ok(Entry {
                path: PathBuf::from("LICENSE"),
                keywords: vec![
                    Keyword::Size(10931),
                    Keyword::Time(DateTime::from_timestamp(1769203027, 452198079).unwrap()),
                    Keyword::Sha256(
                        "014bb31e83d5c2e76aea1cc6e82217346ab41362f32cb355ad0f5c10aa0aeaff"
                            .to_string()
                    )
                ]
            })
        );
    }
}
