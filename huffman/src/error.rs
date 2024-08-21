use std::fmt::Display;

#[derive(Debug, Clone)]
pub enum Error {
    FileUnreadable,
    FileWriting,
    NotEnoughDifferentChars,
    UsingOWithoutFile,
    BadOption,
    NoFileProvided,
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::FileUnreadable => write!(f, "The provided file is unreadable"),
            Error::NotEnoughDifferentChars => write!(
                f,
                "To be compressed, the file needs at least 2 distinct characters"
            ),
            Error::UsingOWithoutFile => {
                write!(f, "You must provide a filename if you use option -o.")
            }
            Error::BadOption => write!(f, "Options not recognized, please check the usage."),
            Error::FileWriting => write!(f, "An error occured while writing to the output file"),
            Error::NoFileProvided => write!(f, "No file provided."),
        }
    }
}
