//! A minimal client for the local node, over plain HTTP/1.1 on loopback (standard library only).
//!
//! Used by `chxchx-tech node status|stop`, and later by Studio to attach to the node (H5).
//! It only ever talks to `127.0.0.1`.

use std::io::{Read, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpStream};
use std::time::Duration;

pub struct Response {
    pub status: u16,
    pub body: String,
}

/// Sends one request to the node on `port` and reads the whole answer.
pub fn request(
    port: u16,
    method: &str,
    path: &str,
    token: Option<&str>,
    body: Option<&str>,
) -> Result<Response, String> {
    let address = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
    let mut stream = TcpStream::connect_timeout(&address, Duration::from_secs(2))
        .map_err(|error| format!("El nodo no responde en {address}: {error}"))?;
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .and_then(|()| stream.set_write_timeout(Some(Duration::from_secs(10))))
        .map_err(|error| error.to_string())?;
    let body = body.unwrap_or_default();
    let mut head = format!(
        "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\nContent-Length: {}\r\n",
        body.len()
    );
    if !body.is_empty() {
        head.push_str("Content-Type: application/json\r\n");
    }
    if let Some(token) = token {
        head.push_str(&format!("Authorization: Bearer {token}\r\n"));
    }
    head.push_str("\r\n");
    stream
        .write_all(head.as_bytes())
        .and_then(|()| stream.write_all(body.as_bytes()))
        .map_err(|error| format!("No se pudo hablar con el nodo: {error}"))?;
    let mut raw = Vec::new();
    stream
        .read_to_end(&mut raw)
        .map_err(|error| format!("El nodo no terminó de responder: {error}"))?;
    parse(&raw)
}

fn parse(raw: &[u8]) -> Result<Response, String> {
    let text = String::from_utf8_lossy(raw);
    let (head, body) = text
        .split_once("\r\n\r\n")
        .ok_or("Respuesta del nodo incompleta")?;
    let status = head
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|code| code.parse().ok())
        .ok_or("Respuesta del nodo sin estado")?;
    let chunked = head
        .lines()
        .any(|line| line.eq_ignore_ascii_case("transfer-encoding: chunked"));
    let body = if chunked {
        dechunk(body)
    } else {
        body.to_owned()
    };
    Ok(Response { status, body })
}

fn dechunk(mut rest: &str) -> String {
    let mut out = String::new();
    while let Some((size, after)) = rest.split_once("\r\n") {
        let Ok(size) = usize::from_str_radix(size.trim(), 16) else {
            break;
        };
        if size == 0 || after.len() < size {
            break;
        }
        out.push_str(&after[..size]);
        rest = after[size..].trim_start_matches("\r\n");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_and_chunked_answers_are_read() {
        let plain = parse(b"HTTP/1.1 200 OK\r\ncontent-length: 2\r\n\r\n{}").unwrap();
        assert_eq!((plain.status, plain.body.as_str()), (200, "{}"));
        let chunked = parse(
            b"HTTP/1.1 202 Accepted\r\ntransfer-encoding: chunked\r\n\r\n4\r\n{\"a\"\r\n3\r\n:1}\r\n0\r\n\r\n",
        )
        .unwrap();
        assert_eq!((chunked.status, chunked.body.as_str()), (202, "{\"a\":1}"));
        assert!(parse(b"garbage").is_err());
    }
}
