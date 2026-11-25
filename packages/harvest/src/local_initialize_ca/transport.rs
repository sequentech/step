// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use super::{file, json, Registration};
use crate::routes::private_operational_ca::spki;
use anyhow::{ensure, Context, Result};
use openssl::asn1::Asn1Time;
use openssl::pkey::PKey;
use openssl::stack::Stack;
use openssl::x509::store::X509StoreBuilder;
use openssl::x509::verify::{X509VerifyFlags, X509VerifyParam};
use openssl::x509::{X509PurposeId, X509StoreContext, X509};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::net::IpAddr;
use std::path::Path;
use windmill::services::signing::certificates::is_ca;
use zeroize::Zeroizing;

#[derive(Copy, Clone, Deserialize, Eq, PartialEq)]
#[serde(try_from = "String")]
pub(super) struct Pin([u8; 32]);

impl TryFrom<String> for Pin {
    type Error = &'static str;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let mut bytes = [0u8; 32];
        if value.bytes().any(|b| b.is_ascii_uppercase())
            || hex::decode_to_slice(value, &mut bytes).is_err()
            || bytes == [0; 32]
        {
            return Err("expected a lowercase nonzero SHA-256 pin");
        }
        Ok(Self(bytes))
    }
}

// Wire-compatible with Beyond's broker and device role registrations.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ClientPins {
    harvest_to_broker: Pin,
    broker_to_device: Pin,
    device_to_ca: Pin,
}

impl ClientPins {
    fn distinct(&self, server: Pin, server_ca: Pin, client_ca: Pin) -> bool {
        let keys = [
            server,
            server_ca,
            client_ca,
            self.harvest_to_broker,
            self.broker_to_device,
            self.device_to_ca,
        ];
        keys.iter()
            .enumerate()
            .all(|(i, key)| !keys[..i].contains(key))
    }
}

pub(super) struct Transport {
    pub(super) chain: Vec<u8>,
    pub(super) key: Zeroizing<Vec<u8>>,
    pub(super) client_ca: Vec<u8>,
    pub(super) device_spki: [u8; 32],
}

fn anchor(pem: &[u8], expected: Pin) -> Result<X509> {
    let mut certificates = X509::stack_from_pem(pem)?;
    ensure!(certificates.len() == 1, "expected a single transport CA");
    let cert = certificates.pop().context("missing transport CA")?;
    let actual = Pin(Sha256::digest(cert.to_der()?).into());
    ensure!(actual == expected, "transport CA digest mismatch");
    let now = Asn1Time::days_from_now(0)?;
    ensure!(
        is_ca(&cert)? && cert.not_before() <= &*now && cert.not_after() > &*now,
        "expected a current transport CA"
    );
    Ok(cert)
}

impl Transport {
    pub(super) fn read(
        dir: &Path,
        registration: &Registration,
    ) -> Result<Self> {
        let pins: ClientPins = json(&dir.join("client-pins.json"))?;
        let server_ca = anchor(
            &file(&dir.join("server-ca.pem"), false)?,
            registration.server_ca_sha256,
        )?;
        let client_ca = anchor(
            &file(&dir.join("client-ca.pem"), false)?,
            registration.client_ca_sha256,
        )?;
        let chain = file(&dir.join("server.pem"), false)?;
        let key = Zeroizing::new(file(&dir.join("server-key.pem"), true)?);
        let certs = X509::stack_from_pem(&chain)?;
        let (leaf, intermediates) = certs
            .split_first()
            .context("missing CA server certificate")?;
        let server_spki = Pin(spki(leaf)?);
        ensure!(
            server_spki == registration.server_spki_sha256,
            "CA server SPKI mismatch"
        );
        ensure!(!is_ca(leaf)?, "expected a transport server leaf");
        ensure!(
            pins.distinct(
                server_spki,
                Pin(spki(&server_ca)?),
                Pin(spki(&client_ca)?)
            ),
            "transport roles share a key"
        );
        let private_key = PKey::private_key_from_pem_passphrase(&key, b"")?;
        ensure!(
            leaf.public_key()?.public_eq(&private_key),
            "CA server key mismatch"
        );

        // The device trusts only its installed server CA and connects to the
        // IP literal. Verify that same path, purpose, time and IP SAN here.
        let mut store = X509StoreBuilder::new()?;
        store.add_cert(server_ca)?;
        let mut param = X509VerifyParam::new()?;
        param.set_flags(X509VerifyFlags::PARTIAL_CHAIN)?;
        param.set_purpose(X509PurposeId::SSL_SERVER)?;
        param.set_ip(IpAddr::V4(*registration.listen.ip()))?;
        store.set_param(&param)?;
        let mut untrusted = Stack::new()?;
        for cert in intermediates {
            untrusted.push(cert.clone())?;
        }
        let verified = X509StoreContext::new()?.init(
            &store.build(),
            leaf,
            &untrusted,
            |ctx| {
                if !ctx.verify_cert()? {
                    return Ok(false);
                }
                let Some(path) = ctx.chain() else {
                    return Ok(false);
                };
                // The wire chain may omit its trust anchor. Refuse extra,
                // duplicate or out-of-order certificates OpenSSL did not use.
                if path.len() != certs.len() && path.len() != certs.len() + 1 {
                    return Ok(false);
                }
                for (sent, verified) in certs.iter().zip(path.iter()) {
                    if sent.to_der()? != verified.to_der()? {
                        return Ok(false);
                    }
                }
                Ok(true)
            },
        )?;
        ensure!(
            verified,
            "CA server chain does not verify at the installed address"
        );
        // Supply Rocket with precisely the client trust anchor checked above.
        // Its mandatory handshake proves the client chain. The original
        // assessment handler pins the live client leaf's SPKI on every POST.
        Ok(Self {
            chain,
            key,
            client_ca: client_ca.to_pem()?,
            device_spki: pins.device_to_ca.0,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn malformed_and_overlapping_role_pins_are_refused() {
        assert!(Pin::try_from("ab".repeat(32)).is_ok());
        for text in [
            "00".repeat(32),
            "AB".repeat(32),
            "1".repeat(63),
            "xx".repeat(32),
        ] {
            assert!(Pin::try_from(text).is_err());
        }
        let separate = |keys: [u8; 6]| {
            let [server, server_ca, client_ca, harvest_to_broker, broker_to_device, device_to_ca] =
                keys.map(|byte| Pin([byte; 32]));
            ClientPins {
                harvest_to_broker,
                broker_to_device,
                device_to_ca,
            }
            .distinct(server, server_ca, client_ca)
        };
        let keys = [1, 2, 3, 4, 5, 6];
        assert!(separate(keys));
        for i in 0..keys.len() {
            for j in 0..i {
                let mut reused = keys;
                reused[i] = reused[j];
                assert!(!separate(reused));
            }
        }
        assert!(anchor(b"invalid certificate", Pin([1; 32])).is_err());
    }
}
