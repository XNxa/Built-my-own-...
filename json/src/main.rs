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

fn tokenize(input: String) -> Result<Vec<Token>, Error> {
    let mut tokens = Vec::new();
    let mut iter = input.chars();
    while let Some(ch) = iter.next() {
        match ch {
            '{' => tokens.push(Token::OpenBracket),
            '}' => tokens.push(Token::CloseBracket),
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
            '\u{0020}' | '\u{000A}' | '\u{000D}' | '\u{0009}' => continue, // Ignore whitespaces, tabs, ...
            _ => return Err(Error::UnrecognizedToken(ch)),
        }
    }
    Ok(tokens)
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
            Token::OpenBracket => Err(Error::SyntaxError),
            Token::CloseBracket => Ok(object),
            Token::Comma => Err(Error::TrailingComma),
            Token::Colon => Err(Error::SyntaxError),
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
        },
        None => Err(Error::MissingClosingBracket),
    }
}

fn parse_kv(key: String, iter: &mut dyn Iterator<Item = Token>) -> Result<KV, Error> {
    match iter.next() {
        Some(Token::Colon) => match iter.next() {
            Some(Token::Litteral(value)) => Ok(KV(key, Value::Str(value))),
            _ => Err(Error::SyntaxError),
        },
        _ => Err(Error::SyntaxError),
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
}
