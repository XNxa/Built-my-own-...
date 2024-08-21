use crate::error::Error;

pub enum Mode {
    Compress,
    Uncompress,
}

pub struct Args {
    pub input: String,
    pub output: String,
    pub mode: Mode,
}

impl Args {
    pub fn build() -> Result<Self, Error> {
        let args: Vec<String> = std::env::args().collect();
        let mut in_file = None;
        let mut out_file = None;
        let mut mode = Mode::Compress;

        let mut iter = args.iter();
        while let Some(arg) = iter.next() {
            if arg.starts_with("-") {
                match arg.as_str() {
                    "-c" => mode = Mode::Compress,
                    "-u" => mode = Mode::Uncompress,
                    "-o" => match iter.next() {
                        Some(s) => out_file = Some(s.to_string()),
                        None => return Err(Error::UsingOWithoutFile),
                    },
                    _ => return Err(Error::BadOption),
                }
            } else {
                in_file = Some(arg.to_string());
            }
        }

        match in_file {
            Some(filename) => Ok(Args {
                input: filename,
                output: out_file.map_or("a.out".to_string(), |s| s),
                mode,
            }),
            None => Err(Error::NoFileProvided),
        }
    }
}
