mod args;
mod error;
mod huffman;

use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::process::exit;

use args::Args;
use error::Error;
use huffman::HuffmanTree;

fn usage() {
    eprintln!("Usage: huffman [COMMAND] <filename>");
    eprintln!("COMMANDS : ");
    eprintln!("\t-c          : Compress the file <filename>. Default.");
    eprintln!("\t-u          : Uncompress the file <filename>");
    eprintln!("\t-o <output> : Place the result in the specified file. Default to a.out");
}

fn main() {
    let args = match Args::build() {
        Ok(args) => args,
        Err(e) => {
            eprintln!("Error : {}", e);
            usage();
            exit(1);
        }
    };

    match run(args) {
        Ok(s) => {
            println!("{}", s);
            exit(0);
        }
        Err(e) => {
            eprintln!("Error : {}", e);
            exit(1);
        }
    }
}

fn run(args: Args) -> Result<String, Error> {
    let freqs = get_frequencies(&args)?;
    let huffman_tree = if let Some(t) = huffman::HuffmanTree::build_huffman(freqs.clone()) {
        t
    } else {
        return Err(Error::NotEnoughDifferentChars);
    };

    let codes = HuffmanTree::generate_prefix_codes(huffman_tree);

    write_header(args.output.clone(), freqs)?;

    let mut buf: Vec<u8> = Vec::new();
    let mut bit_buffer = Vec::new();
    let mut bit_count = 0u8;

    let mut output_file = OpenOptions::new()
        .append(true)
        .open(args.output)
        .map_err(|_| Error::FileWriting)?;

    for_chars(args.input.clone(), |c| {
        let bits = codes.get(&c).unwrap();
        for bit in bits.chars() {
            if bit == '1' {
                bit_buffer.push(1);
            } else {
                bit_buffer.push(0);
            }
            bit_count += 1;

            if bit_count == 8 {
                buf.push(bit_buffer.iter().fold(0, |acc, b| (acc << 1) | b));
                bit_buffer.clear();
                bit_count = 0;
            }

            if buf.len() > 2048 {
                let _ = output_file.write_all(&buf);
                buf.clear();
            }
        }
        Ok(())
    })?;

    if bit_count > 0 {
        buf.push(bit_buffer.iter().fold(0, |acc, b| (acc << 1) | b));
    }

    if buf.len() > 0 {
        output_file
            .write_all(&buf)
            .map_err(|_| Error::FileWriting)?;
        buf.clear();
    }

    Ok("Ok".to_string())
}

fn write_header(filename: String, freqs: HashMap<char, i32>) -> Result<(), Error> {
    let mut freq_bytes: Vec<u8> = Vec::new();
    for (c, f) in freqs {
        let mut buf = [0; 4];
        let encoded_char = c.encode_utf8(&mut buf);
        freq_bytes.extend(encoded_char.len().to_le_bytes());
        freq_bytes.extend_from_slice(encoded_char.as_bytes());
        freq_bytes.extend(f.to_be_bytes());
    }

    let mut output_file = OpenOptions::new()
        .truncate(true)
        .create(true)
        .write(true)
        .open(filename)
        .map_err(|_| Error::FileWriting)?;

    output_file
        .write_all(&freq_bytes.len().to_be_bytes())
        .map_err(|_| Error::FileWriting)?;

    output_file
        .write_all(&freq_bytes)
        .map_err(|_| Error::FileWriting)?;

    Ok(())
}

fn get_frequencies(args: &Args) -> Result<HashMap<char, i32>, Error> {
    let mut frequencies: HashMap<char, i32> = HashMap::new();

    for_chars(args.input.clone(), |c| {
        *frequencies.entry(c).or_insert(0) += 1;
        Ok(())
    })?;

    Ok(frequencies)
}

fn for_chars<F>(filename: String, mut f: F) -> Result<(), Error>
where
    F: FnMut(char) -> Result<(), Error>,
{
    // Open file
    let mut file = fs::File::open(filename).map_err(|_| Error::FileUnreadable)?;

    let mut buf = [0; 2048];
    let mut left_overs: Vec<u8> = Vec::new();

    while let Ok(amount_read) = file.read(&mut buf) {
        if amount_read == 0 {
            break;
        }

        let mut chunk = left_overs.clone();
        chunk.extend_from_slice(&buf[..amount_read]);

        match std::str::from_utf8(&chunk) {
            Ok(valid_str) => {
                for c in valid_str.chars() {
                    f(c)?;
                }
                left_overs.clear();
            }
            Err(e) => {
                let valid_up_to = e.valid_up_to();
                if valid_up_to > 0 {
                    for c in std::str::from_utf8(&chunk[..valid_up_to]).unwrap().chars() {
                        f(c)?;
                    }
                }
                left_overs = chunk[valid_up_to..].to_vec();
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{args::Mode, get_frequencies, Args};

    #[test]
    fn test_frequencies() {
        let args = Args {
            input: "test.txt".to_string(),
            output: "a.out".to_string(),
            mode: Mode::Compress,
        };
        let freq = get_frequencies(&args).unwrap();

        assert_eq!(*freq.get(&'X').unwrap(), 333);
        assert_eq!(*freq.get(&'t').unwrap(), 223000);
    }
}
