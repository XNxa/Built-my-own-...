use std::io::{stdin, BufReader, Read};
use std::{fs::File, io};

pub enum Input {
    Stdin,
    File(File),
}

impl Read for Input {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            Input::Stdin => stdin().read(buf),
            Input::File(file) => file.read(buf),
        }
    }
}

pub struct Args {
    pub input: BufReader<Input>,
    pub sep: char,
    pub fields: Vec<usize>,
}

impl Args {
    pub fn parse(args: Vec<String>) -> Args {
        let mut iter = args.iter().skip(1).peekable();
        let mut input = Input::Stdin;
        let mut sep = '\t';
        let mut fields: Vec<usize> = vec![];
        while let Some(arg) = iter.next() {
            if arg.starts_with("-f") {
                if arg.len() > 2 {
                    // Parse comma separated values
                    arg.clone()
                        .split_off(2)
                        .split(",")
                        .for_each(|v| fields.push(v.parse().expect(&usage("Invalid col value"))));
                } else {
                    // Parse whitespace separated values
                    if let Some(arg) = iter.next() {
                        arg.clone().split(" ").for_each(|v| {
                            fields.push(v.parse().expect(&usage("Invalid col value")))
                        });
                    } else {
                        panic!("{}", &usage("Expect a list a values after -f"))
                    }
                }
            } else if arg.starts_with("-d") {
                sep = arg
                    .clone()
                    .split_off(2)
                    .chars()
                    .next()
                    .expect(&usage("Please provide a char after -d"))
            } else if arg == "-" {
                input = Input::Stdin
            } else {
                if let Input::File(_) = input {
                    panic!("{}", &usage("Multiple files given"))
                }
                input = Input::File(std::fs::File::open(arg).expect(&usage("File not found")))
            }
        }
        Args {
            input: BufReader::new(input),
            sep,
            fields,
        }
    }
}

fn usage(error: &str) -> String {
    format!(
        "Error: {error}
Usage: cut <option> <filename>
    options:
        -f[a,b,...,c] | -f [\"a b c\"] : Choose cols to extract
        -d[ch]: Set the char delimiter to be ch
\n"
    )
}
