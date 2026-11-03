// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Test-only PostgreSQL wire observer. Only statement text is retained, never
//! Bind parameters, credentials, or server replies. No server settings change.
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

#[derive(Clone, Default)]
pub struct Statements(Arc<Mutex<Option<Vec<String>>>>);
impl Statements {
    pub fn start(&self) {
        *self.0.lock().unwrap() = Some(Vec::new());
    }
    pub fn finish(&self) -> Vec<String> {
        self.0.lock().unwrap().take().unwrap()
    }
    fn record(&self, sql: &str) {
        if let Some(statements) = self.0.lock().unwrap().as_mut() {
            statements.push(sql.into());
        }
    }
}

fn field(bytes: &[u8], offset: &mut usize) -> Option<String> {
    let end = bytes.get(*offset..)?.iter().position(|byte| *byte == 0)? + *offset;
    let text = std::str::from_utf8(&bytes[*offset..end]).ok()?.to_owned();
    *offset = end + 1;
    Some(text)
}

async fn forward(
    client: TcpStream,
    server: TcpStream,
    observed: Statements,
) -> std::io::Result<()> {
    let (mut cr, mut cw) = client.into_split();
    let (mut sr, mut sw) = server.into_split();
    let upstream = async move {
        // StartupMessage is the only frontend frame without its one-byte tag.
        // Test connections use NoTls, so there is no encrypted traffic to inspect.
        let length = cr.read_u32().await?;
        if !(8..=65536).contains(&length) {
            return Err(std::io::Error::other("invalid startup length"));
        }
        let mut startup = vec![0; length as usize - 4];
        cr.read_exact(&mut startup).await?;
        sw.write_u32(length).await?;
        sw.write_all(&startup).await?;
        let mut prepared = HashMap::new();
        let mut portals = HashMap::new();
        loop {
            let tag = match cr.read_u8().await {
                Ok(tag) => tag,
                Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(()),
                Err(error) => return Err(error),
            };
            let length = cr.read_u32().await?;
            if !(4..=16 * 1024 * 1024).contains(&length) {
                return Err(std::io::Error::other("invalid frame length"));
            }
            let mut body = vec![0; length as usize - 4];
            cr.read_exact(&mut body).await?;
            let mut offset = 0;
            match tag {
                b'P' => {
                    if let (Some(name), Some(sql)) =
                        (field(&body, &mut offset), field(&body, &mut offset))
                    {
                        prepared.insert(name, sql);
                    }
                }
                b'B' => {
                    if let (Some(portal), Some(name)) =
                        (field(&body, &mut offset), field(&body, &mut offset))
                    {
                        if let Some(sql) = prepared.get(&name) {
                            portals.insert(portal, sql.clone());
                        }
                    }
                }
                b'E' => {
                    if let Some(portal) = field(&body, &mut offset) {
                        if let Some(sql) = portals.get(&portal) {
                            observed.record(sql);
                        }
                    }
                }
                b'Q' => {
                    if let Some(sql) = field(&body, &mut offset) {
                        observed.record(&sql);
                    }
                }
                _ => {}
            }
            sw.write_u8(tag).await?;
            sw.write_u32(length).await?;
            sw.write_all(&body).await?;
        }
    };
    let downstream = async move { tokio::io::copy(&mut sr, &mut cw).await.map(|_| ()) };
    tokio::try_join!(upstream, downstream)?;
    Ok(())
}

pub async fn start(host: String, port: u16) -> (u16, Statements, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
        .await
        .unwrap();
    let local_port = listener.local_addr().unwrap().port();
    let observed = Statements::default();
    let shared = observed.clone();
    let task = tokio::spawn(async move {
        while let Ok((client, _)) = listener.accept().await {
            let host = host.clone();
            let observed = shared.clone();
            tokio::spawn(async move {
                let server = TcpStream::connect((host.as_str(), port)).await.unwrap();
                client.set_nodelay(true).unwrap();
                server.set_nodelay(true).unwrap();
                let _ = forward(client, server, observed).await;
            });
        }
    });
    (local_port, observed, task)
}
