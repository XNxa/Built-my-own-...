use std::{env, io::BufRead};

use args::Args;

mod args;

fn main() {
    let mut args = Args::parse(env::args().collect());

    let mut buf = String::new();
    while let Ok(bytes_read) = args.input.read_line(&mut buf) {
        if bytes_read == 0 {
            break;
        }

        let mut col = 1;
        for val in buf.split(args.sep) {
            if args.fields.contains(&col) {
                if col == 1 {
                    print!("{val}")
                } else {
                    print!("\t{val}")
                }
            }
            col += 1;
        }
        print!("\n");
        buf.clear();
    }
}
