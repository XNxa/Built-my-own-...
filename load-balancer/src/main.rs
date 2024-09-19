use std::io::Read;
use std::io::Write;
use std::{
    io::{BufRead, BufReader},
    net::{TcpListener, TcpStream},
    thread,
};

#[derive(Debug)]
struct Error(String);

macro_rules! error {
    ($s:expr) => {
        Error(format!("{:?}", $s))
    };
}

macro_rules! debug {
    ($s:expr) => {
        println!("DEBUG: {}", $s)
    };
}

fn read_http_request(reader: &mut impl Read) -> Result<String, Error> {
    let mut reader = BufReader::new(reader);
    let mut req = String::new();
    let mut buf = String::new();

    // Parse first line
    reader.read_line(&mut buf).map_err(|e| error!(e.kind()))?;

    if !buf.to_lowercase().contains("http/1.1") {
        return Err(error!("This server only handle HTTP 1.1"));
    }
    req.push_str(&buf);
    buf.clear();

    // Parse Header
    let mut content_len = None;
    while let Ok(_) = reader.read_line(&mut buf) {
        let keyword = "content-length:";
        if buf.to_lowercase().trim().starts_with(keyword) {
            content_len = Some(
                buf[keyword.len()..]
                    .trim()
                    .parse::<usize>()
                    .map_err(|e| error!(e.kind()))?,
            )
        }

        // If we encounter an empty line, we've reached the end of the headers
        if buf == "\r\n" || buf == "\n" {
            req.push_str("\r\n");
            break;
        }

        req.push_str(&buf);
        buf.clear();
    }

    // Parse optional content
    if let Some(len) = content_len {
        let mut buf = vec![0u8; len];
        reader
            .read_exact(&mut buf)
            .map_err(|e| error!(format!("Invalid header : {}", e.kind())))?;

        req.push_str(&String::from_utf8(buf).map_err(|_| error!("Body is not valid utf8"))?);
    }

    Ok(req)
}

fn handle_conn(mut conn: TcpStream, server: &str) {
    let mut serv = TcpStream::connect(server).unwrap();

    let req = read_http_request(&mut conn).unwrap();

    write!(serv, "{}", req).unwrap();

    let req = read_http_request(&mut serv).unwrap();

    write!(conn, "{}", req).unwrap();
}

fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:5050")?;

    let mut round_robin = ["localhost:8080", "localhost:8081", "localhost:8082"]
        .iter()
        .cycle();

    for conn in listener.incoming() {
        let server = *round_robin.next().unwrap();
        let _thread_handle = thread::spawn(move || {
            handle_conn(conn.unwrap(), server);
        });
    }

    Ok(())
}
