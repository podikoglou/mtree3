use std::path::PathBuf;

use chrono::{DateTime, Utc};
use chumsky::prelude::*;

pub struct Entry {
    pub path: String,
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
    any()
        .repeated()
        .to_slice()
        .validate(|x: &str, _, _| PathBuf::from(x))
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

    choice((
        just("type")
            .ignore_then(just("="))
            .ignore_then(type_value)
            .map(|ty| Keyword::Type(ty)),
        just("uid")
            .ignore_then(just("="))
            .ignore_then(number_u32)
            .map(|uid| Keyword::Uid(uid)),
        just("time")
            .ignore_then(just("="))
            .ignore_then(timestamp)
            .map(|time| Keyword::Time(time)),
        just("size")
            .ignore_then(just("="))
            .ignore_then(number_u64)
            .map(|size| Keyword::Size(size)),
        choice((just("sha256digest"), just("sha256")))
            .ignore_then(just("="))
            .ignore_then(text::ident())
            .map(|sha256: &str| Keyword::Sha256(sha256.to_string())),
        just("link")
            .ignore_then(just("="))
            .ignore_then(path)
            .map(|path: PathBuf| Keyword::Link(path)),
    ))
}

pub fn parse_keywords<'src>() -> impl Parser<'src, &'src str, Vec<Keyword>> {
    parse_keyword().separated_by(text::whitespace()).collect()
}

pub fn parse_command<'src>() -> impl Parser<'src, &'src str, Command> {
    let unset = just("unset").to(Command::Unset);
    let set = just("set")
        .ignore_then(text::whitespace())
        .ignore_then(parse_keywords())
        .map(Command::Set);

    just('/')
        .ignore_then(choice((unset, set)))
        .then_ignore(end()) // <- not sure if this is needed, it may even break stuff
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

    #[test]
    fn test_parse_path() {
        assert_eq!(
            parse_path().parse("foo.bar").into_result(),
            Ok(PathBuf::from("foo.bar"))
        );
        assert_eq!(
            parse_path().parse("../../foo.bar").into_result(),
            Ok(PathBuf::from("../../foo.bar"))
        );
    }

    #[test]
    fn test_parse_type_keyword() {
        assert_eq!(
            parse_keyword().parse("type=block").into_result(),
            Ok(Keyword::Type(Type::Block))
        );
        assert_eq!(
            parse_keyword().parse("type=char").into_result(),
            Ok(Keyword::Type(Type::Char))
        );
        assert_eq!(
            parse_keyword().parse("type=dir").into_result(),
            Ok(Keyword::Type(Type::Dir))
        );
        assert_eq!(
            parse_keyword().parse("type=fifo").into_result(),
            Ok(Keyword::Type(Type::Fifo))
        );
        assert_eq!(
            parse_keyword().parse("type=file").into_result(),
            Ok(Keyword::Type(Type::File))
        );
        assert_eq!(
            parse_keyword().parse("type=link").into_result(),
            Ok(Keyword::Type(Type::Link))
        );
        assert_eq!(
            parse_keyword().parse("type=socket").into_result(),
            Ok(Keyword::Type(Type::Socket))
        );
    }

    #[test]
    fn test_parse_uid_keyword() {
        assert_eq!(
            parse_keyword().parse("uid=0").into_result(),
            Ok(Keyword::Uid(0))
        );
        assert_eq!(
            parse_keyword().parse("uid=100").into_result(),
            Ok(Keyword::Uid(100))
        );
        assert_eq!(
            parse_keyword().parse("uid=123456789").into_result(),
            Ok(Keyword::Uid(123456789))
        );
    }

    #[test]
    fn test_parse_timestamp_keyword() {
        assert_eq!(
            parse_keyword().parse("time=1630456800.0").into_result(),
            Ok(Keyword::Time(
                DateTime::from_timestamp(1630456800, 0).unwrap()
            ))
        );
        assert_eq!(
            parse_keyword()
                .parse("time=1769640177.434772208")
                .into_result(),
            Ok(Keyword::Time(
                DateTime::from_timestamp(1769640177, 434772208).unwrap()
            ))
        );
    }

    #[test]
    fn test_parse_size_keyword() {
        assert_eq!(
            parse_keyword().parse("size=0").into_result(),
            Ok(Keyword::Size(0))
        );
        assert_eq!(
            parse_keyword().parse("size=1024").into_result(),
            Ok(Keyword::Size(1024))
        );
        assert_eq!(
            parse_keyword().parse("size=1048576").into_result(),
            Ok(Keyword::Size(1048576))
        );
    }

    #[test]
    fn test_parse_sha256_keyword() {
        assert_eq!(
            parse_keyword()
                .parse("sha256=fd9849d9364b9b9aabed88a8aa8e007d7450c3ad3a17aee0617dd24959464249")
                .into_result(),
            Ok(Keyword::Sha256(
                "fd9849d9364b9b9aabed88a8aa8e007d7450c3ad3a17aee0617dd24959464249".to_string()
            ))
        );
        assert_eq!(
            parse_keyword()
                .parse(
                    "sha256digest=fd9849d9364b9b9aabed88a8aa8e007d7450c3ad3a17aee0617dd24959464249"
                )
                .into_result(),
            Ok(Keyword::Sha256(
                "fd9849d9364b9b9aabed88a8aa8e007d7450c3ad3a17aee0617dd24959464249".to_string()
            ))
        );
    }

    #[test]
    fn test_parse_link_keyword() {
        assert_eq!(
            parse_keyword().parse("link=../../foo.bar").into_result(),
            Ok(Keyword::Link(PathBuf::from("../../foo.bar")))
        );
        assert_eq!(
            parse_keyword().parse("link=./foo.bar").into_result(),
            Ok(Keyword::Link(PathBuf::from("./foo.bar")))
        );
        assert_eq!(
            parse_keyword().parse("link=foo.bar").into_result(),
            Ok(Keyword::Link(PathBuf::from("foo.bar")))
        );
    }

    #[test]
    fn test_parse_keywords() {
        assert_eq!(parse_keywords().parse("").into_result(), Ok(vec![]));

        assert_eq!(
            parse_keywords().parse("type=dir").into_result(),
            Ok(vec![Keyword::Type(Type::Dir),])
        );

        assert_eq!(
            parse_keywords()
                .parse("type=dir size=384 time=1769640373.412526597")
                .into_result(),
            Ok(vec![
                Keyword::Type(Type::Dir),
                Keyword::Size(384),
                Keyword::Time(DateTime::from_timestamp(1769640373, 412526597).unwrap())
            ])
        );

        assert_eq!(
            parse_keywords()
                .parse("type=link size=24 time=1769203307.589764008")
                .into_result(),
            Ok(vec![
                Keyword::Type(Type::Link),
                Keyword::Size(24),
                Keyword::Time(DateTime::from_timestamp(1769203307, 589764008).unwrap())
            ])
        );
    }

    #[test]
    fn test_commands() {
        assert_eq!(
            parse_command()
                .parse("/set type=dir size=384 time=1769640373.412526597")
                .into_result(),
            Ok(Command::Set(vec![
                Keyword::Type(Type::Dir),
                Keyword::Size(384),
                Keyword::Time(DateTime::from_timestamp(1769640373, 412526597).unwrap())
            ]))
        );

        assert_eq!(
            parse_command().parse("/unset").into_result(),
            Ok(Command::Unset)
        );
    }
}
