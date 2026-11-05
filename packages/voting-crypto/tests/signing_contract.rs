// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use base64::{engine::general_purpose::STANDARD, Engine as _};
use voting_crypto::signing::{Signature, SigningKey, VerificationKey};
use voting_crypto::{RecordDeserialize, RecordSerialize};

#[test]
fn rfc8032_empty_message_vector_and_standard_public_key() {
    let seed =
        hex::decode("9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60").unwrap();
    let sk = SigningKey::from_b64_string(&STANDARD.encode(seed)).unwrap();
    let pk = VerificationKey::from_sk(&sk).unwrap();
    assert_eq!(
        hex::encode(pk.to_bytes()),
        "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a"
    );
    let signature = sk.sign(b"").unwrap();
    assert_eq!(hex::encode(signature.to_bytes()), "e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e065224901555fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b");
    pk.verify(&signature, b"").unwrap();
    assert!(pk.verify(&signature, b"changed").is_err());
    let spki = hex::decode(
        "302a300506032b6570032100d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a",
    )
    .unwrap();
    assert_eq!(pk.to_der().unwrap(), spki);
    assert_eq!(
        VerificationKey::from_b64_string(&STANDARD.encode(spki)).unwrap(),
        pk
    );
    let pkcs8 = hex::decode("302e020100300506032b6570042204209d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60").unwrap();
    assert_eq!(
        SigningKey::from_b64_string(&STANDARD.encode(pkcs8))
            .unwrap()
            .to_bytes(),
        sk.to_bytes()
    );
}

#[test]
fn signed_application_records_keep_raw_borsh_and_hex_json() {
    let sk = SigningKey::from_b64_string(&STANDARD.encode([7u8; 32])).unwrap();
    let pk = VerificationKey::from_sk(&sk).unwrap();
    let signature = sk.sign(b"record").unwrap();
    assert_eq!(pk.record_serialize().unwrap(), pk.to_bytes());
    assert_eq!(signature.record_serialize().unwrap(), signature.to_bytes());
    assert_eq!(
        serde_json::to_string(&pk).unwrap(),
        format!("\"{}\"", hex::encode(pk.to_bytes()))
    );
    let parsed: Signature =
        serde_json::from_str(&serde_json::to_string(&signature).unwrap()).unwrap();
    pk.verify(&parsed, b"record").unwrap();
    let mut extended = pk.record_serialize().unwrap();
    extended.push(0);
    assert!(VerificationKey::record_deserialize(&extended).is_err());
}
