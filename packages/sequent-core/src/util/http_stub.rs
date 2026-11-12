// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use std::io::{BufRead, BufReader, Read, Result, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

/// Minimal HTTP/1.1 server on a local port. Each request is answered with the
/// status and JSON body returned by `respond`, given the request line. The
/// server thread stops when the stub is dropped.
pub(crate) struct HttpStub {
    url: String,
    address: SocketAddr,
    stopped: Arc<AtomicBool>,
}

impl HttpStub {
    pub(crate) fn start<F>(respond: F) -> Result<Self>
    where
        F: Fn(&str) -> (u16, String) + Send + 'static,
    {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let address = listener.local_addr()?;
        let stopped = Arc::new(AtomicBool::new(false));
        let stop = Arc::clone(&stopped);
        thread::spawn(move || {
            for stream in listener.incoming() {
                if stop.load(Ordering::SeqCst) {
                    break;
                }
                if let Ok(stream) = stream {
                    let _ = answer(stream, &respond);
                }
            }
        });
        Ok(Self {
            url: format!("http://{address}"),
            address,
            stopped,
        })
    }

    pub(crate) fn url(&self) -> &str {
        &self.url
    }
}

impl Drop for HttpStub {
    /// Flags the server thread to stop, then connects once to wake it from
    /// the blocking accept.
    fn drop(&mut self) {
        self.stopped.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect(self.address);
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
