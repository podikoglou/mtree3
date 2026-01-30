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

pub fn parse_keyword<'src>() -> impl Parser<'src, &'src str, Keyword> {
    let type_value = parse_type();

    let number_u32 = text::int::<_, extra::Err<EmptyErr>>(10)
        .to_slice()
        .try_map(|s: &str, _| s.parse::<u32>().map_err(|_| EmptyErr::default()));

    let number_u64 = text::int::<_, extra::Err<EmptyErr>>(10)
        .to_slice()
        .try_map(|s: &str, _| s.parse::<u64>().map_err(|_| EmptyErr::default()));

    let timestamp = parse_timestamp();

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
            .ignore_then(text::ident()) // <-- this may not be right
            .map(|sha256: &str| Keyword::Sha256(sha256.to_string())),
        just("link")
            .ignore_then(just("="))
            .ignore_then(text::ident()) // <-- this is likely not right
            .map(|sha256: &str| Keyword::Link(sha256.to_string())),
    ))
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
            Ok(Keyword::Link("../../foo.bar".to_string()))
        );
        assert_eq!(
            parse_keyword().parse("link=./foo.bar").into_result(),
            Ok(Keyword::Link("./foo.bar".to_string()))
        );
        assert_eq!(
            parse_keyword().parse("link=foo.bar").into_result(),
            Ok(Keyword::Link("foo.bar".to_string()))
        );
    }
}
