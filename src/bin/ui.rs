use std::io::{Read, Write};
use std::net::TcpStream;

fn request(path: &str) -> String {
    let mut stream = TcpStream::connect(("127.0.0.1", 8080)).unwrap();
    let req = format!("GET {} HTTP/1.1\r\nHost: localhost\r\n\r\n", path);
    stream.write_all(req.as_bytes()).unwrap();
    let mut bytes = Vec::new();
    stream.read_to_end(&mut bytes).unwrap();
    String::from_utf8_lossy(&bytes).to_string()
}

fn main() {
    let _ = request("/cmd?insert=hello");
    let resp = request("/state");
    println!("{resp}");
}
