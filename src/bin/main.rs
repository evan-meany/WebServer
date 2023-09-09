use std::fs;
use std::net::TcpListener;
use std::net::TcpStream;
use std::io::prelude::*;
use std::thread;
use std::time::Duration;
use web_server::ThreadPool;

fn main() {
    // 127.0.0.1 is local host ip address and 7878 is the port number 
    let listener = 
        TcpListener::bind("127.0.0.1:7878").unwrap();
    let pool = ThreadPool::new(4);
    
    for stream in listener.incoming() {
        let stream = stream.unwrap();
        pool.execute(|| { handle_connection(stream); });
    }
}

fn handle_connection(mut stream: TcpStream) {
    // This buffer is 1024 bytes long
    let mut buffer = [0; 1024];
    stream.read(&mut buffer).unwrap();
    // println!("Buffer: {}", String::from_utf8_lossy(&buffer[..]));
    
    let home = b"GET / HTTP/1.1\r\n";
    let sleep = b"GET /sleep HTTP/1.1\r\n";
    let (status_line, filename) = 
        if buffer.starts_with(home) {
            ("HTTP/1.1 200 OK", "index.html")
        } else if buffer.starts_with(sleep) {
            thread::sleep(Duration::from_secs(5));
            ("HTTP/1.1 200 OK", "index.html")
        } else {
            ("HTTP/1.1 404 NOT FOUND", "404.html")
        };

    let contents = fs::read_to_string(filename).unwrap();

    // response format:
    // HTTP-Version Status-Code Reason-Phrase CRLF
    // headers CRLF (Character Return Line Feed sequence - \r\n)
    // message-body
    //
    // ex: HTTP/1.1 200 Ok\r\n\r\n [contains no headers/message-body]
    let response = format!(
        "{}\r\nContent-Length: {}\r\n\r\n{}",
        status_line,
        contents.len(),
        contents
    );

    stream.write(response.as_bytes()).unwrap();
    stream.flush().unwrap();
}