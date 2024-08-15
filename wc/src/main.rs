use std::fs;
use std::io::{self, BufRead, Read};
use std::process::exit;

#[derive(Debug)]
enum ErrorMessage {
    FileUnreadable,
    UnknownOption,
    TooManyFiles,
}

impl std::fmt::Display for ErrorMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorMessage::FileUnreadable => write!(f, "Unable to read file"),
            ErrorMessage::UnknownOption => write!(f, "Unknown option"),
            ErrorMessage::TooManyFiles => write!(f, "Too many files are given as input"),
        }
    }
}

enum Mode {
    Bytes,
    Lines,
    Words,
    Chars,
}

fn usage() {
    eprintln!("Usage : wc [options] <file>");
}

struct Args {
    modes: Vec<Mode>,
    filename: Option<String>,
}

impl Args {
    fn from(args: Vec<String>) -> Result<Args, ErrorMessage> {
        let mut modes: Vec<Mode> = Vec::new();
        let mut filename = None;
        for arg in args.iter().skip(1) {
            if arg.starts_with('-') {
                modes.push(match arg.as_str() {
                    "-l" => Mode::Lines,
                    "-c" => Mode::Bytes,
                    "-w" => Mode::Words,
                    "-m" => Mode::Chars,
                    _ => return Err(ErrorMessage::UnknownOption),
                })
            } else {
                filename = match filename {
                    None => Some(arg.clone()),
                    Some(_) => return Err(ErrorMessage::TooManyFiles),
                }
            }
        }

        if modes.is_empty() {
            modes.push(Mode::Words)
        }

        Ok(Args { modes, filename })
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let args = Args::from(args);
    match args {
        Ok(args) => match run(args) {
            Ok(s) => {
                println!("{}", s);
                exit(0);
            }
            Err(e) => {
                usage();
                eprintln!("Error: {}", e);
                exit(1);
            }
        },
        Err(e) => {
            usage();
            eprintln!("Error: {}", e);
            exit(1);
        }
    }
}

fn run(args: Args) -> Result<String, ErrorMessage> {
    let input: Box<dyn BufRead> = if let Some(filepath) = args.filename {
        let file = fs::File::open(&filepath).map_err(|_| ErrorMessage::FileUnreadable)?;
        Box::new(io::BufReader::new(file))
    } else {
        Box::new(io::BufReader::new(io::stdin()))
    };

    for mode in args.modes.iter() {
        match mode {
            Mode::Bytes => {
                let mut buf = Vec::new();
                let mut reader = input.take(usize::MAX as u64);
                reader
                    .read_to_end(&mut buf)
                    .map_err(|_| ErrorMessage::FileUnreadable)?;
                return Ok(format!("{}", buf.len()));
            }
            Mode::Lines => {
                return Ok(format!("{}", input.lines().count()));
            }
            Mode::Words => {
                let word_count = input.lines().fold(0, |acc, e| {
                    acc + e
                        .unwrap()
                        .split_whitespace()
                        .filter(|s| !s.is_empty())
                        .count()
                });
                return Ok(format!("{}", word_count));
            }
            Mode::Chars => {
                return handle_chars(input);
            }
        }
    }

    Ok(String::new())
}

fn handle_chars<R: BufRead>(mut reader: R) -> Result<String, ErrorMessage> {
    let mut buf = [0; 2048];
    let mut chars_count = 0;
    let mut left_overs: Vec<u8> = Vec::new();

    while let Ok(bytes_read) = reader.read(&mut buf) {
        if bytes_read == 0 {
            break;
        }

        let mut chunk = left_overs.clone();
        chunk.extend_from_slice(&buf[..bytes_read]);

        match std::str::from_utf8(&chunk) {
            Ok(valid_str) => {
                chars_count += valid_str.chars().count();
                left_overs.clear();
            }
            Err(e) => {
                let valid_up_to = e.valid_up_to();
                if valid_up_to > 0 {
                    chars_count += std::str::from_utf8(&chunk[..valid_up_to])
                        .unwrap()
                        .chars()
                        .count();
                }
                left_overs = chunk[valid_up_to..].to_vec();
            }
        }
    }

    Ok(format!("{}", chars_count))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nofile() {
        let result = run(Args {
            modes: vec![Mode::Bytes],
            filename: Some("pas_la.pasla".to_string()),
        });

        assert!(matches!(result, Err(ErrorMessage::FileUnreadable)));
    }

    #[test]
    fn test_c() {
        let result = run(Args {
            modes: vec![Mode::Bytes],
            filename: Some("test.txt".to_string()),
        });

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "342190".to_string());
    }

    #[test]
    fn test_l() {
        let result = run(Args {
            modes: vec![Mode::Lines],
            filename: Some("test.txt".to_string()),
        });

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "7145".to_string());
    }

    #[test]
    fn test_1l() {
        let result = run(Args {
            modes: vec![Mode::Lines],
            filename: Some("1.txt".to_string()),
        });

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "1".to_string());
    }

    #[test]
    fn test_0l() {
        let result = run(Args {
            modes: vec![Mode::Lines],
            filename: Some("0.txt".to_string()),
        });

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "0".to_string());
    }

    #[test]
    fn test_w() {
        let result = run(Args {
            modes: vec![Mode::Words],
            filename: Some("test.txt".to_string()),
        });

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "58164".to_string());
    }

    #[test]
    fn test_m() {
        let result = run(Args {
            modes: vec![Mode::Chars],
            filename: Some("test.txt".to_string()),
        });

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "339292".to_string());
    }
}
