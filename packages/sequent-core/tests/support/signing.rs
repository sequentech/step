// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Bearer tokens signed with the public test key of the verification fixtures,
//! and the issuer settings that make the request guards accept them. A local
//! server stands in for the realm's certificate endpoint, so the guards run
//! their real signature, issuer and expiry checks.

use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use sequent_core::services::connection::BearerIssuers;
use serde_json::Value;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::OnceLock;
use std::time::Duration;

pub const ISSUER: &str = "https://identity.invalid";
/// Far enough ahead that fixtures never expire during a run.
pub const EXPIRY: u64 = 4_102_444_800;

const KEY_ID: &str = "test-key";
const KEY: &[u8] =
    include_bytes!("../../src/services/jwt_verification/fixtures/test-key.pem");
const KEYS: &str =
    include_str!("../../src/services/jwt_verification/fixtures/test-jwks.json");

/// The issuer of tokens from the realm of `tenant_id`.
pub fn issuer_of(tenant_id: &str) -> String {
    format!("{ISSUER}/realms/tenant-{tenant_id}")
}

pub fn token(claims: &Value) -> String {
    let mut header = Header::new(Algorithm::RS256);
    header.kid = Some(KEY_ID.into());
    encode(&header, claims, &EncodingKey::from_rsa_pem(KEY).unwrap()).unwrap()
}

/// Trusts [`ISSUER`] and downloads its keys from the local server.
pub fn issuers() -> BearerIssuers {
    BearerIssuers {
        internal_base: key_server().into(),
        trusted_bases: vec![ISSUER.into()],
    }
}

fn key_server() -> &'static str {
    static URL: OnceLock<String> = OnceLock::new();
    URL.get_or_init(|| {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                std::thread::spawn(move || answer(stream));
            }
        });
        url
    })
}

fn answer(mut stream: TcpStream) {
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut line = String::new();
    while reader.read_line(&mut line).is_ok_and(|read| read > 2) {
        line.clear();
    }
    let _ = write!(
        stream,
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{KEYS}",
        KEYS.len()
    );
}
