// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use std::io::{BufRead, BufReader, Read, Result, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;

/// Minimal HTTP/1.1 server on a local port. Each request is answered with the
/// status and JSON body returned by `respond`, given the request line.
pub(crate) struct HttpStub {
    url: String,
}

impl HttpStub {
    pub(crate) fn start<F>(respond: F) -> Result<Self>
    where
        F: Fn(&str) -> (u16, String) + Send + 'static,
    {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let url = format!("http://{}", listener.local_addr()?);
        thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let _ = answer(stream, &respond);
            }
        });
        Ok(Self { url })
    }

    pub(crate) fn url(&self) -> &str {
        &self.url
    }
}

fn answer<F>(mut stream: TcpStream, respond: &F) -> Result<()>
where
    F: Fn(&str) -> (u16, String),
{
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut request_line = String::new();
    reader.read_line(&mut request_line)?;

    let mut content_length = 0;
    loop {
        let mut header = String::new();
        if reader.read_line(&mut header)? == 0 || header == "\r\n" {
            break;
        }
        if let Some((name, value)) = header.split_once(':') {
            if name.eq_ignore_ascii_case("content-length") {
                content_length = value.trim().parse().unwrap_or(0);
            }
        }
    }
    let mut body = vec![0; content_length];
    reader.read_exact(&mut body)?;

    let (status, response_body) = respond(request_line.trim_end());
    write!(
        stream,
        "HTTP/1.1 {status} STUB\r\nContent-Type: application/json\r\n\
         Content-Length: {}\r\nConnection: close\r\n\r\n{response_body}",
        response_body.len()
    )?;
    stream.flush()
}
