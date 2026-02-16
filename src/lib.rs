use std::path::PathBuf;

use chumsky::prelude::*;
use jiff::Timestamp;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub path: PathBuf,
    pub keywords: Vec<Keyword>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Keyword {
    Type(Type),
    Uid(u32),
    Time(Timestamp),
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

pub type ParserError<'src> = extra::Err<Rich<'src, char>>;

pub fn parse_type<'src>() -> impl Parser<'src, &'src str, Type, ParserError<'src>> {
    choice((
        just("block").to(Type::Block),
        just("char").to(Type::Char),
        just("dir").to(Type::Dir),
        just("fifo").to(Type::Fifo),
        just("file").to(Type::File),
        just("link").to(Type::Link),
        just("socket").to(Type::Socket),
    ))
    .labelled("type")
}

pub fn parse_timestamp<'src>() -> impl Parser<'src, &'src str, Timestamp, ParserError<'src>> {
    // NOTE: do we need to handle - for the first number? (since it's an i64)
    // let number = just('0').repeated().or_not().then(text::int(10)).to_slice();

    // TODO: clean this up, this is horrid
    let number = choice((
        // example: 1111
        text::int(10).repeated().to_slice(),
        // example: 0
        just('0').to_slice(),
        // example: 01111
        just('0')
            .to_slice()
            .then(text::int(10).repeated().to_slice())
            .to_slice(),
    ));

    number
        .then(just('.'))
        .then(number)
        .to_slice()
        .try_map(|str: &str, span: SimpleSpan| {
            Timestamp::strptime("%s.%f", str)
                .map_err(|err| Rich::custom(span, format!("Can't parse timestamp: {}", err)))
        })
}

pub fn parse_path<'src>() -> impl Parser<'src, &'src str, PathBuf, ParserError<'src>> {
    none_of(" \t") // <-- NOTE: this will backfire
        .repeated()
        .at_least(1)
        .to_slice()
        .validate(|x: &str, _, _| PathBuf::from(x))
        .labelled("path")
}

fn keyword_parser<'src, V>(
    key: impl Parser<'src, &'src str, &'src str, ParserError<'src>>,
    value: impl Parser<'src, &'src str, V, ParserError<'src>>,
) -> impl Parser<'src, &'src str, V, ParserError<'src>> {
    key.ignore_then(just('='))
        .ignore_then(value)
        .labelled("keyword")
}

pub fn parse_keyword<'src>() -> impl Parser<'src, &'src str, Keyword, ParserError<'src>> {
    let type_value = parse_type();

    let number_u32 = text::int(10).map(|s: &str| s.parse::<u32>().unwrap());
    let number_u64 = text::int(10).map(|s: &str| s.parse::<u64>().unwrap());

    let timestamp = parse_timestamp();

    let path = parse_path();

    let sha256 = one_of("0123456789abcdefABCDEF")
        .repeated()
        .at_least(1)
        .to_slice()
        .labelled("sha256 hash");

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

pub fn whitespace_with_continuation<'src>() -> impl Parser<'src, &'src str, (), ParserError<'src>> {
    choice((
        text::whitespace().map(|_| ()),
        just('\\')
            .then(text::newline())
            .then(text::whitespace().or_not())
            .map(|_| ()),
    ))
}

pub fn parse_keywords<'src>() -> impl Parser<'src, &'src str, Vec<Keyword>, ParserError<'src>> {
    parse_keyword()
        .separated_by(whitespace_with_continuation())
        .collect()
}

pub fn parse_command<'src>() -> impl Parser<'src, &'src str, Command, ParserError<'src>> {
    let unset = just("unset").to(Command::Unset);
    let set = just("set")
        .ignore_then(whitespace_with_continuation())
        .ignore_then(parse_keywords())
        .map(Command::Set);

    just('/')
        .ignore_then(choice((unset, set)))
        .then_ignore(end()) // <- not sure if this is needed, it may even break stuff
        .labelled("command")
}

pub fn parse_comment<'src>() -> impl Parser<'src, &'src str, (), ParserError<'src>> {
    just('#')
        .ignore_then(any().repeated())
        .ignore_then(choice((text::newline().ignore_then(end()), end()))) // <-- this may or may not be totally useles
        .ignored()
}

pub fn parse_entry<'src>() -> impl Parser<'src, &'src str, Entry, ParserError<'src>> {
    let path = parse_path();
    let keywords = parse_keywords();

    path.padded_by(whitespace_with_continuation())
        .then(keywords)
        .map(|(path, keywords)| Entry { path, keywords })
        .labelled("entry")
}

pub fn parse_entries<'src>() -> impl Parser<'src, &'src str, Vec<Entry>, ParserError<'src>> {
    parse_entry()
        .separated_by(text::newline())
        .at_least(1)
        .collect::<Vec<_>>()
        .then_ignore(text::whitespace().or_not())
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
            Ok(Timestamp::strptime("%s.%f", "1630456800.0").unwrap())
        );
        assert_eq!(
            parse("1630456800.01"),
            Ok(Timestamp::strptime("%s.%f", "1630456800.01").unwrap())
        );
        assert_eq!(
            parse("1769640177.434772208"),
            Ok(Timestamp::strptime("%s.%f", "1769640177.434772208").unwrap())
        );
    }

    #[test]
    fn test_parse_path() {
        let parse = |input| parse_path().parse(input).into_result();

        assert_eq!(parse("."), Ok(PathBuf::from(".")));
        assert_eq!(parse("foo.bar"), Ok(PathBuf::from("foo.bar")));
        assert_eq!(parse("../../foo.bar"), Ok(PathBuf::from("../../foo.bar")));
        assert_eq!(parse("3"), Ok(PathBuf::from("3")));
        assert_eq!(parse("0.txt"), Ok(PathBuf::from("0.txt")));
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
                Timestamp::strptime("%s.%f", "1630456800.0").unwrap()
            ))
        );
        assert_eq!(
            parse("time=1769640177.434772208"),
            Ok(Keyword::Time(
                Timestamp::strptime("%s.%f", "1769640177.434772208").unwrap()
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
                Keyword::Time(Timestamp::strptime("%s.%f", "1769640373.412526597").unwrap())
            ])
        );

        assert_eq!(
            parse("type=link size=24 time=1769203307.589764008"),
            Ok(vec![
                Keyword::Type(Type::Link),
                Keyword::Size(24),
                Keyword::Time(Timestamp::strptime("%s.%f", "1769203307.589764008").unwrap())
            ])
        );

        assert_eq!(
            parse(
                "size=10931 time=1769203027.452198079 \
                                sha256digest=014bb31e83d5c2e76aea1cc6e82217346ab41362f32cb355ad0f5c10aa0aeaff"
            ),
            Ok(vec![
                Keyword::Size(10931),
                Keyword::Time(Timestamp::strptime("%s.%f", "1769203027.452198079").unwrap()),
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
                Keyword::Time(Timestamp::strptime("%s.%f", "1769640373.412526597").unwrap())
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
                    Keyword::Time(Timestamp::strptime("%s.%f", "1769203027.452198079").unwrap())
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
                    Keyword::Time(Timestamp::strptime("%s.%f", "1769203027.452198079").unwrap()),
                    Keyword::Sha256(
                        "014bb31e83d5c2e76aea1cc6e82217346ab41362f32cb355ad0f5c10aa0aeaff"
                            .to_string()
                    )
                ]
            })
        );
    }

    #[test]
    fn test_parse_entries() {
        let parse = |input| parse_entries().parse(input).into_result();

        assert_eq!(
            parse(
                ".             type=dir size=320 time=1771023429.226137224
                .gitignore  size=8 time=1769725259.452161299
                Cargo.lock  size=13637 time=1769728676.006587414
                Cargo.toml  size=114 time=1769728674.520418961
                LICENSE     size=1066 time=1769784305.767405992
                README.md   size=177 time=1769783557.896055811
                "
            ),
            Ok(vec![
                Entry {
                    path: PathBuf::from("."),
                    keywords: vec![
                        Keyword::Type(Type::Dir),
                        Keyword::Size(320),
                        Keyword::Time(
                            Timestamp::strptime("%s.%f", "1771023429.226137224").unwrap()
                        )
                    ]
                },
                Entry {
                    path: PathBuf::from(".gitignore"),
                    keywords: vec![
                        Keyword::Size(8),
                        Keyword::Time(
                            Timestamp::strptime("%s.%f", "1769725259.452161299").unwrap()
                        )
                    ]
                },
                Entry {
                    path: PathBuf::from("Cargo.lock"),
                    keywords: vec![
                        Keyword::Size(13637),
                        Keyword::Time(
                            Timestamp::strptime("%s.%f", "1769728676.006587414").unwrap()
                        )
                    ]
                },
                Entry {
                    path: PathBuf::from("Cargo.toml"),
                    keywords: vec![
                        Keyword::Size(114),
                        Keyword::Time(
                            Timestamp::strptime("%s.%f", "1769728674.520418961").unwrap()
                        )
                    ]
                },
                Entry {
                    path: PathBuf::from("LICENSE"),
                    keywords: vec![
                        Keyword::Size(1066),
                        Keyword::Time(
                            Timestamp::strptime("%s.%f", "1769784305.767405992").unwrap()
                        )
                    ]
                },
                Entry {
                    path: PathBuf::from("README.md"),
                    keywords: vec![
                        Keyword::Size(177),
                        Keyword::Time(
                            Timestamp::strptime("%s.%f", "1769783557.896055811").unwrap()
                        )
                    ]
                }
            ])
        );
    }
}
