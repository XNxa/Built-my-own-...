mod args;
mod error;
mod huffman;

use core::str;
use std::cmp::max;
use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::Read;
use std::io::Write;
use std::process::exit;

use args::Args;
use error::Error;
use huffman::HuffmanTree;

type FreqTable = HashMap<char, u32>;

fn usage() {
    eprintln!("Usage: huffman [COMMAND] <filename>");
    eprintln!("COMMANDS : ");
    eprintln!("\t-c          : Compress the file <filename>. Default.");
    eprintln!("\t-d          : Decompress the file <filename>");
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

    match args.mode {
        args::Mode::Compress => match encode(args) {
            Ok(s) => {
                println!("{}", s);
                exit(0);
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                exit(1);
            }
        },
        args::Mode::Decompress => match decode(args) {
            Ok(s) => println!("{}", s),
            Err(e) => {
                eprintln!("Error: {}", e);
                exit(1);
            }
        },
    }
}

fn encode(args: Args) -> Result<String, Error> {
    let freqs = get_frequencies(&args)?;
    let huffman_tree = if let Some(t) = huffman::HuffmanTree::build_huffman(freqs.clone()) {
        t
    } else {
        return Err(Error::NotEnoughDifferentChars);
    };

    let codes = HuffmanTree::gen_char_code_map(huffman_tree);

    let mut output_file = OpenOptions::new()
        .truncate(true)
        .create(true)
        .write(true)
        .open(args.output)
        .map_err(|_| Error::FileWriting)?;

    let mut input_file = File::open(args.input).map_err(|_| Error::FileUnreadable)?;
    write_header(&mut output_file, freqs)?;
    write_encoded_file(codes, &mut input_file, &mut output_file)?;

    Ok("Ok".to_string())
}

fn write_encoded_file(
    codes: HashMap<char, String>,
    input_file: &mut File,
    output_file: &mut File,
) -> Result<(), Error> {
    let mut buf: Vec<u8> = Vec::new();
    let mut bit_buffer = Vec::new();

    let input_file = input_file;
    let output_file = output_file;

    let original_size = input_file
        .metadata()
        .map_err(|_| Error::FileUnreadable)?
        .len();

    output_file
        .write(&original_size.to_le_bytes())
        .map_err(|_| Error::FileWriting)?;

    for_chars(input_file, |c| {
        let bits = codes.get(&c).unwrap();
        for bit in bits.chars() {
            bit_buffer.push(if bit == '1' { 1 } else { 0 });

            if bit_buffer.len() == 8 {
                buf.push(bit_buffer.iter().fold(0, |acc, b| (acc << 1) | *b));
                bit_buffer.clear();
            }

            if buf.len() >= 2048 {
                let _ = output_file.write_all(&buf);
                buf.clear();
            }
        }
        Ok(())
    })?;

    if bit_buffer.len() > 0 {
        let mut last_byte = bit_buffer.iter().fold(0, |acc, b| (acc << 1) | *b);
        last_byte = last_byte << 8 - bit_buffer.len();
        buf.push(last_byte);
        bit_buffer.clear();
    }

    if buf.len() > 0 {
        output_file
            .write_all(&buf)
            .map_err(|_| Error::FileWriting)?;
        buf.clear();
    }

    Ok(())
}

fn decode(args: Args) -> Result<String, Error> {
    let mut file = fs::File::open(args.input).map_err(|_| Error::FileUnreadable)?;
    let mut output_file = fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(args.output)
        .map_err(|_| Error::InvalidFile)?;

    let table = read_header(&mut file)?;

    let huffman_tree = if let Some(t) = huffman::HuffmanTree::build_huffman(table) {
        t
    } else {
        return Err(Error::NotEnoughDifferentChars);
    };

    let codes = HuffmanTree::gen_code_char_map(huffman_tree);
    write_decoded_file(codes, &mut file, &mut output_file)?;

    Ok("".to_string())
}

fn write_decoded_file(
    codes: HashMap<String, char>,
    input_file: &mut File,
    output_file: &mut File,
) -> Result<(), Error> {
    let file = input_file;
    let output_file = output_file;

    let mut nb_of_bytes = [0u8; 8];
    file.read_exact(&mut nb_of_bytes)
        .map_err(|_| Error::InvalidFile)?;
    let nb_of_bytes = u64::from_le_bytes(nb_of_bytes);

    let max_len_code = codes.iter().fold(0, |acc, e| max(acc, e.0.len()));

    let mut buf = [0; 2048];
    let mut current_prefix = "".to_string();
    let mut decoded_chars = Vec::new();
    let mut bytes_decoded = 0;
    while let Ok(n) = file.read(&mut buf) {
        if n == 0 {
            break;
        }
        for i in 0..n {
            let byte = buf[i];
            for j in (0..8).rev() {
                current_prefix.push(if (byte >> j) & 1 == 1 { '1' } else { '0' });
                if let Some(c) = codes.get(&current_prefix) {
                    decoded_chars.push(*c);
                    bytes_decoded += 1;
                    current_prefix.clear();
                    if bytes_decoded == nb_of_bytes {
                        break;
                    }
                }
            }
            if current_prefix.len() > max_len_code {
                return Err(Error::InvalidFile);
            }
        }

        let out: String = decoded_chars.iter().map(|c| String::from(*c)).collect();
        output_file
            .write_all(out.as_bytes())
            .map_err(|_| Error::FileWriting)?;
        decoded_chars.clear();
    }
    Ok(())
}

/// Write the frequency table to the beginning of the file following this format :
///
/// - 4 bytes integer : indicating the nb of bytes for the rest of this header
/// - for entries in table :
///     - 1 byte integer  : length (n) of char
///     - n bytes         : character
///     - 4 bytes integer : frequency
fn write_header(file: &mut File, freqs: FreqTable) -> Result<(), Error> {
    let mut freq_bytes: Vec<u8> = Vec::new();
    for (c, f) in freqs {
        let mut buf = [0; 4];
        let encoded_char = c.encode_utf8(&mut buf);
        freq_bytes.extend((encoded_char.len() as u8).to_le_bytes());
        freq_bytes.extend_from_slice(encoded_char.as_bytes());
        freq_bytes.extend(f.to_le_bytes());
    }

    let output_file = file;

    output_file
        .write_all(&(freq_bytes.len() as u32).to_le_bytes())
        .map_err(|_| Error::FileWriting)?;

    output_file
        .write_all(&freq_bytes)
        .map_err(|_| Error::FileWriting)?;

    Ok(())
}

fn read_header(file: &mut File) -> Result<FreqTable, Error> {
    let file = file;

    let mut header_size_len = [0u8; 4];
    file.read_exact(&mut header_size_len)
        .map_err(|_| Error::FileReading)?;
    let header_size_len = u32::from_le_bytes(header_size_len);

    let mut header = vec![0u8; header_size_len as usize];
    file.read_exact(&mut header)
        .map_err(|_| Error::FileReading)?;

    let mut table = FreqTable::new();

    let mut iter = header.iter();
    while let Some(b) = iter.next() {
        let char_size = u8::from_le_bytes([*b]);
        let mut char_buf = vec![0; char_size as usize];
        for i in 0..char_size {
            char_buf[i as usize] = match iter.next() {
                Some(b) => *b,
                None => return Err(Error::InvalidFile),
            }
        }
        let char = match str::from_utf8(&char_buf)
            .map_err(|_| Error::InvalidFile)?
            .chars()
            .nth(0)
        {
            Some(c) => c,
            None => return Err(Error::InvalidFile),
        };

        let mut f_buf = [0; 4];
        for i in 0..4 {
            f_buf[i] = match iter.next() {
                Some(b) => *b,
                None => return Err(Error::InvalidFile),
            }
        }
        let freq = u32::from_le_bytes(f_buf);

        table.insert(char, freq);
    }
    Ok(table)
}

fn get_frequencies(args: &Args) -> Result<FreqTable, Error> {
    let mut frequencies: FreqTable = HashMap::new();

    let mut file = File::open(args.input.clone()).map_err(|_| Error::FileUnreadable)?;
    for_chars(&mut file, |c| {
        *frequencies.entry(c).or_insert(0) += 1;
        Ok(())
    })?;

    Ok(frequencies)
}

fn for_chars<F>(file: &mut File, mut f: F) -> Result<(), Error>
where
    F: FnMut(char) -> Result<(), Error>,
{
    let file = file;
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
    use std::{
        collections::HashMap,
        fs::{read_to_string, remove_file, File, OpenOptions},
        io::{Read, Write},
    };

    use crate::{
        args::Mode, decode, encode, for_chars, get_frequencies, huffman::HuffmanTree, read_header,
        write_decoded_file, write_encoded_file, write_header, Args, FreqTable,
    };

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

    #[test]
    fn test_for_chars() {
        let mut n = 0;
        let mut file = File::open("test.txt").unwrap();
        for_chars(&mut file, |_| Ok(n += 1)).unwrap();
        assert_eq!(n, 3324222);
    }

    #[test]
    fn test_for_chars_2() {
        let mut file = File::open("test.txt").unwrap();
        let mut file_copy = OpenOptions::new()
            .truncate(true)
            .create(true)
            .write(true)
            .open("test_copy.test")
            .unwrap();

        for_chars(&mut file, |c| {
            let mut buf = [0u8; 4];
            let s = c.encode_utf8(&mut buf);
            Ok(file_copy.write(s.as_bytes()).map(|_| ()).unwrap())
        })
        .unwrap();

        let mut s1 = String::new();
        let mut s2 = String::new();
        File::open("test.txt")
            .unwrap()
            .read_to_string(&mut s1)
            .unwrap();
        File::open("test_copy.test")
            .unwrap()
            .read_to_string(&mut s2)
            .unwrap();

        assert_eq!(s1, s2);

        remove_file("test_copy.test").unwrap();
    }

    #[test]
    fn test_header() {
        let mut freqs = FreqTable::new();
        freqs.insert('a', 10);

        let path = "test_header.txt";

        let mut f = OpenOptions::new()
            .truncate(true)
            .create(true)
            .write(true)
            .open(path)
            .unwrap();

        write_header(&mut f, freqs.clone()).unwrap();

        let mut f = File::open(path).unwrap();
        let freqs_read = read_header(&mut f).unwrap();

        assert_eq!(freqs.len(), freqs_read.len());
        assert_eq!(freqs.get(&'a').unwrap(), freqs_read.get(&'a').unwrap());

        remove_file(path).unwrap();
    }

    #[test]
    fn test_header_2() {
        let mut freqs = FreqTable::new();
        freqs.insert('a', 10);
        freqs.insert('\n', 100000);
        freqs.insert('\u{feff}', 800000);

        let path = "test_header2.txt";

        let mut f = OpenOptions::new()
            .truncate(true)
            .create(true)
            .write(true)
            .open(path)
            .unwrap();

        write_header(&mut f, freqs.clone()).unwrap();

        let mut f = File::open(path).unwrap();
        let freqs_read = read_header(&mut f).unwrap();

        assert_eq!(freqs.len(), freqs_read.len());
        assert_eq!(freqs.get(&'a').unwrap(), freqs_read.get(&'a').unwrap());
        assert_eq!(freqs.get(&'\n').unwrap(), freqs_read.get(&'\n').unwrap());
        assert_eq!(
            freqs.get(&'\u{feff}').unwrap(),
            freqs_read.get(&'\u{feff}').unwrap()
        );

        remove_file(path).unwrap();
    }

    #[test]
    fn test_encode() {
        let path = "test_encode.test";
        let path2 = "test_encode2.test";
        {
            let mut file = OpenOptions::new()
                .create(true)
                .truncate(true)
                .write(true)
                .open(path)
                .unwrap();

            let mut file2 = OpenOptions::new()
                .create(true)
                .truncate(true)
                .write(true)
                .open(path2)
                .unwrap();

            let mut codes = HashMap::new();
            codes.insert('a', "1".to_string());
            codes.insert('\n', "0".to_string());

            write!(file, "a\naaa").unwrap();
            file.flush().unwrap();
            let mut file = File::open(path).unwrap();
            write_encoded_file(codes, &mut file, &mut file2).unwrap();
        }
        let mut f = File::open(path2).unwrap();
        let mut buf = [0; 9];
        f.read_exact(&mut buf).unwrap();

        assert_eq!(184, buf[8]);
        remove_file(path).unwrap();
        remove_file(path2).unwrap();
    }

    #[test]
    fn test_decode() {
        let path = "test_decode.test";
        let path2 = "test_decode2.test";
        {
            let mut file = OpenOptions::new()
                .create(true)
                .truncate(true)
                .write(true)
                .open(path)
                .unwrap();

            let mut file2 = OpenOptions::new()
                .create(true)
                .truncate(true)
                .write(true)
                .open(path2)
                .unwrap();

            let mut codes = HashMap::new();
            codes.insert("1".to_string(), 'a');
            codes.insert("0".to_string(), '\n');

            file.write(&5u64.to_le_bytes()).unwrap();
            file.write(&[184]).unwrap();
            file.flush().unwrap();
            let mut file = File::open(path).unwrap();
            write_decoded_file(codes, &mut file, &mut file2).unwrap();
        }
        let mut f = File::open(path2).unwrap();
        let mut buf = [0; 5];
        f.read_exact(&mut buf).unwrap();

        assert_eq!("a\naaa".as_bytes(), buf);
        remove_file(path).unwrap();
        remove_file(path2).unwrap();
    }

    #[test]
    fn test_encode_non_ascii() {
        let path = "test_encode_non_ascii.test";
        let path2 = "test_encode2_non_ascii.test";
        {
            let mut file = OpenOptions::new()
                .create(true)
                .truncate(true)
                .write(true)
                .open(path)
                .unwrap();

            let mut file2 = OpenOptions::new()
                .create(true)
                .truncate(true)
                .write(true)
                .open(path2)
                .unwrap();

            let mut codes = HashMap::new();
            codes.insert('é', "1".to_string());
            codes.insert('$', "0".to_string());

            write!(file, "é$ééé").unwrap();
            file.flush().unwrap();
            let mut file = File::open(path).unwrap();
            write_encoded_file(codes, &mut file, &mut file2).unwrap();
        }
        let mut f = File::open(path2).unwrap();
        let mut buf = [0; 9];
        f.read_exact(&mut buf).unwrap();

        assert_eq!(184, buf[8]);
        remove_file(path).unwrap();
        remove_file(path2).unwrap();
    }

    #[test]
    fn test_decode_non_ascii() {
        let path = "test_decode_non_ascii.test";
        let path2 = "test_decode2_non_ascii.test";
        {
            let mut file = OpenOptions::new()
                .create(true)
                .truncate(true)
                .write(true)
                .open(path)
                .unwrap();

            let mut file2 = OpenOptions::new()
                .create(true)
                .truncate(true)
                .write(true)
                .open(path2)
                .unwrap();

            let mut codes = HashMap::new();
            codes.insert("1".to_string(), 'é');
            codes.insert("0".to_string(), '$');

            file.write(&5u64.to_le_bytes()).unwrap();
            file.write(&[184]).unwrap();
            file.flush().unwrap();
            let mut file = File::open(path).unwrap();
            write_decoded_file(codes, &mut file, &mut file2).unwrap();
        }
        let mut f = File::open(path2).unwrap();
        let mut buf = [0; 9];
        f.read_exact(&mut buf).unwrap();

        assert_eq!("é$ééé".as_bytes(), buf);
        remove_file(path).unwrap();
        remove_file(path2).unwrap();
    }

    #[test]
    fn full_test() {
        let in_path = "full.test";
        let out_path = "full_recovered.test";
        {
            let mut file = OpenOptions::new()
                .create(true)
                .truncate(true)
                .write(true)
                .open(in_path)
                .unwrap();
            file.write("àéÔ%*$abcd1234([][][][][);".as_bytes()).unwrap();
        }

        let args = Args {
            input: in_path.to_string(),
            output: "temp.test".to_string(),
            mode: Mode::Compress,
        };
        let freq1 = get_frequencies(&args).unwrap();
        encode(args).unwrap();

        let args = Args {
            input: "temp.test".to_string(),
            output: out_path.to_string(),
            mode: Mode::Uncompress,
        };
        decode(args).unwrap();

        let s1 = read_to_string(in_path).unwrap();
        let s2 = read_to_string(out_path).unwrap();

        let huff1 = HuffmanTree::build_huffman(freq1).unwrap();

        let mut f = File::open("temp.test").unwrap();
        let freq2 = read_header(&mut f).unwrap();
        let huff2 = HuffmanTree::build_huffman(freq2).unwrap();

        let mut differents = Vec::new();
        let codes = HuffmanTree::gen_char_code_map(huff2);
        for (char, code) in HuffmanTree::gen_char_code_map(huff1) {
            if *codes.get(&char).unwrap() != code {
                differents.push(char);
            }
        }

        assert!(differents.len() == 0, "{:?}", differents);
        assert_eq!(s1, s2);

        remove_file(in_path).unwrap();
        remove_file(out_path).unwrap();
        remove_file("temp.test").unwrap();
    }
}
