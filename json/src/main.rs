use crate::error::Error;
use std::{env, process::exit};

mod error;

#[derive(Debug)]
enum Token {
    OpenBracket,
    CloseBracket,
    Comma,
    Colon,
    Litteral(String),
    True,
    False,
    Null,
    Number(f64),
    OpenList,
    CloseList,
}

#[derive(Debug, PartialEq)]
enum Value {
    Null,
    Bool(bool),
    Number(f64),
    Str(String),
    Array(Vec<Value>),
    Object(Object),
}

type Object = Vec<KV>;

#[derive(PartialEq, Debug)]
struct KV(String, Value);

fn read_end_word(end_of_word: &str, iter: &mut dyn Iterator<Item = char>) -> Result<(), Error> {
    for c in end_of_word.chars() {
        match (c, iter.next()) {
            (a, Some(b)) => {
                if a != b {
                    return Err(Error::UnrecognizedToken(b));
                }
            }
            _ => {
                return Err(Error::ParsingError);
            }
        }
    }
    Ok(())
}

fn tokenize(input: String) -> Result<Vec<Token>, Error> {
    let mut tokens = Vec::new();
    let mut iter = input.chars();
    while let Some(ch) = iter.next() {
        println!("{}", ch);
        match ch {
            '{' => tokens.push(Token::OpenBracket),
            '}' => tokens.push(Token::CloseBracket),
            '[' => tokens.push(Token::OpenList),
            ']' => tokens.push(Token::CloseList),
            ',' => tokens.push(Token::Comma),
            ':' => tokens.push(Token::Colon),
            '"' => {
                let mut l = String::new();
                loop {
                    match iter.next() {
                        Some(c) => {
                            if c == '"' {
                                break;
                            }
                            l.push(c);
                        }
                        None => return Err(Error::MismatchQuote),
                    }
                }
                tokens.push(Token::Litteral(l))
            }
            't' => match read_end_word("rue", &mut iter) {
                Ok(()) => tokens.push(Token::True),
                Err(e) => return Err(e),
            },
            'f' => match read_end_word("alse", &mut iter) {
                Ok(()) => tokens.push(Token::False),
                Err(e) => return Err(e),
            },
            'n' => match read_end_word("ull", &mut iter) {
                Ok(()) => tokens.push(Token::Null),
                Err(e) => return Err(e),
            },
            '\u{0020}' | '\u{000A}' | '\u{000D}' | '\u{0009}' => continue, // Ignore whitespaces, tabs, ...
            c @ '-' | c @ '0'..='9' => match tokenize_digits(c, &mut iter) {
                Ok(n) => tokens.push(Token::Number(n)),
                Err(e) => return Err(e),
            },
            _ => return Err(Error::UnrecognizedToken(ch)),
        }
    }
    Ok(tokens)
}

fn tokenize_digits(c: char, iter: &mut std::str::Chars<'_>) -> Result<f64, Error> {
    let mut iter = iter.peekable();
    let mut s = String::new();
    s.push(c);

    while let Some(ch) = iter.peek() {
        if !"0123456789Ee.+-".contains(*ch) {
            break;
        }
        s.push(iter.next().unwrap())
    }

    s.parse().map_err(|_| Error::InvalidNumber)
}

fn main() -> Result<(), Error> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Please provide a file");
        exit(1);
    }
    let input = std::fs::read_to_string(args[1].clone()).expect("The provided file is unreadable.");

    analyse(input).map(|_| ())
}

fn analyse(raw: String) -> Result<Object, Error> {
    let tokens = tokenize(raw)?;

    let mut iter = tokens.into_iter();
    let json = match iter.next() {
        Some(Token::OpenBracket) => parse_object(&mut iter),
        _ => Err(Error::MustBeginWithBracket),
    }?;

    Ok(json)
}

fn parse_object(iter: &mut dyn Iterator<Item = Token>) -> Result<Object, Error> {
    let mut object = Object::new();
    match iter.next() {
        Some(t) => match t {
            Token::CloseBracket => Ok(object),
            Token::Comma => Err(Error::TrailingComma),
            Token::Litteral(key) => {
                match parse_kv(key, iter) {
                    Ok(kv) => object.push(kv),
                    Err(e) => return Err(e),
                }
                loop {
                    match iter.next() {
                        Some(Token::Comma) => match iter.next() {
                            Some(Token::Litteral(key)) => match parse_kv(key, iter) {
                                Ok(kv) => object.push(kv),
                                Err(e) => return Err(e),
                            },
                            _ => return Err(Error::SyntaxError),
                        },
                        Some(Token::CloseBracket) => return Ok(object),
                        _ => return Err(Error::SyntaxError),
                    }
                }
            }
            _ => Err(Error::SyntaxError),
        },
        None => Err(Error::MissingClosingBracket),
    }
}

fn parse_list(iter: &mut (dyn Iterator<Item = Token>)) -> Result<Value, Error> {
    let mut values = Vec::new();
    match parse_value(iter) {
        Ok(v) => values.push(v),
        Err(e) => return Err(e),
    }
    while let Some(token) = iter.next() {
        match token {
            Token::Comma => match parse_value(iter) {
                Ok(v) => values.push(v),
                Err(e) => return Err(e),
            },
            Token::CloseList => return Ok(Value::Array(values)),
            _ => return Err(Error::SyntaxError),
        }
    }
    return Err(Error::SyntaxError);
}

fn parse_kv(key: String, iter: &mut dyn Iterator<Item = Token>) -> Result<KV, Error> {
    match iter.next() {
        Some(Token::Colon) => parse_value(iter).map(|v| KV(key, v)),
        _ => Err(Error::SyntaxError),
    }
}

fn parse_value(iter: &mut (dyn Iterator<Item = Token>)) -> Result<Value, Error> {
    match iter.next() {
        Some(t) => match t {
            Token::OpenBracket => parse_object(iter).map(|kvs| Value::Object(kvs)),
            Token::Litteral(l) => Ok(Value::Str(l)),
            Token::True => Ok(Value::Bool(true)),
            Token::False => Ok(Value::Bool(false)),
            Token::Null => Ok(Value::Null),
            Token::Number(n) => Ok(Value::Number(n)),
            Token::OpenList => parse_list(iter),
            _ => Err(Error::SyntaxError),
        },
        None => Err(Error::SyntaxError),
    }
}

#[cfg(test)]
mod tests {
    use crate::{analyse, Value, KV};

    #[test]
    fn test_step1_valid() {
        let json = analyse(std::fs::read_to_string("tests/step1/valid.json").unwrap()).unwrap();

        assert!(json.len() == 0);
    }

    #[test]
    fn test_step1_invalid() {
        assert!(analyse(std::fs::read_to_string("tests/step1/invalid.json").unwrap()).is_err());
    }

    #[test]
    fn test_step2_valid() {
        let json = analyse(std::fs::read_to_string("tests/step2/valid.json").unwrap()).unwrap();

        assert_eq!(
            json[0],
            KV("key".to_string(), Value::Str("value".to_string()))
        );
    }

    #[test]
    fn test_step2_valid2() {
        let json = analyse(std::fs::read_to_string("tests/step2/valid2.json").unwrap()).unwrap();

        assert_eq!(
            json[0],
            KV("key".to_string(), Value::Str("value".to_string()))
        );
        assert_eq!(
            json[1],
            KV("key2".to_string(), Value::Str("value".to_string()))
        );
    }

    #[test]
    fn test_step2_invalid() {
        assert!(analyse(std::fs::read_to_string("tests/step2/invalid.json").unwrap()).is_err());
    }

    #[test]
    fn test_step2_invalid2() {
        assert!(analyse(std::fs::read_to_string("tests/step2/invalid2.json").unwrap()).is_err());
    }

    #[test]
    fn test_step3_valid() {
        let json = analyse(std::fs::read_to_string("tests/step3/valid.json").unwrap()).unwrap();

        assert_eq!(json[0], KV("key1".to_string(), Value::Bool(true)));
        assert_eq!(json[1], KV("key2".to_string(), Value::Bool(false)));
        assert_eq!(json[2], KV("key3".to_string(), Value::Null));
        assert_eq!(
            json[3],
            KV("key4".to_string(), Value::Str("value".to_string()))
        );
        assert_eq!(json[4], KV("key5".to_string(), Value::Number(101f64)));
    }

    #[test]
    fn test_step3_invalid() {
        assert!(analyse(std::fs::read_to_string("tests/step3/invalid.json").unwrap()).is_err());
    }

    #[test]
    fn test_step4_valid() {
        let json = analyse(std::fs::read_to_string("tests/step4/valid.json").unwrap()).unwrap();

        assert_eq!(
            json[0],
            KV("key".to_string(), Value::Str("value".to_string()))
        );
        assert_eq!(json[1], KV("key-n".to_string(), Value::Number(101f64)));
        assert_eq!(json[2], KV("key-o".to_string(), Value::Object(Vec::new())));
        assert_eq!(json[3], KV("key-l".to_string(), Value::Array(Vec::new())));
    }

    #[test]
    fn test_step4_valid2() {
        let json = analyse(std::fs::read_to_string("tests/step4/valid2.json").unwrap()).unwrap();

        assert_eq!(
            json[0],
            KV("key".to_string(), Value::Str("value".to_string()))
        );
        assert_eq!(json[1], KV("key-n".to_string(), Value::Number(101f64)));
        assert_eq!(
            json[2],
            KV(
                "key-o".to_string(),
                Value::Object(vec![KV(
                    "inner key".to_string(),
                    Value::Str("inner value".to_string())
                )])
            )
        );
        assert_eq!(
            json[3],
            KV(
                "key-l".to_string(),
                Value::Array(vec![(Value::Str("list value".to_string()))])
            )
        );
    }

    #[test]
    fn test_step4_invalid() {
        assert!(analyse(std::fs::read_to_string("tests/step4/invalid.json").unwrap()).is_err());
    }
}
