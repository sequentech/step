// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Integrity envelope for the complete execution archive. The one-use Ed25519
//! private key remains inside OpenSSL and is freed immediately after signing.
use anyhow::{ensure, Result};
use openssl::{pkey::PKey, sign::Signer};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::{File, OpenOptions},
    io::{Read, Write},
    path::Path,
};
use zip::write::FileOptions;

pub struct ArchiveSignature {
    pub manifest_sha256: String,
    pub public_key: String,
}

pub fn sign(
    path: &Path,
    tenant: &str,
    event: &str,
    document: &str,
    revision: i64,
    generation: i64,
) -> Result<ArchiveSignature> {
    let mut archive = zip::ZipArchive::new(File::open(path)?)?;
    let mut entries = BTreeMap::new();
    for index in 0..archive.len() {
        let mut item = archive.by_index(index)?;
        ensure!(
            !item.name().starts_with("execution-integrity/"),
            "Archive contains a reserved integrity entry"
        );
        let mut digest = Sha256::new();
        let mut buffer = [0_u8; 65536];
        loop {
            let n = item.read(&mut buffer)?;
            if n == 0 {
                break;
            }
            digest.update(&buffer[..n]);
        }
        ensure!(
            entries
                .insert(item.name().to_string(), hex::encode(digest.finalize()))
                .is_none(),
            "Archive contains duplicate file names"
        );
    }
    drop(archive);
    let manifest = serde_json::to_vec_pretty(&json!({"format":"sequent-execution-1.0",
        "tenant_id":tenant,"election_event_id":event,"document_id":document,
        "source_revision":revision,"lifecycle_generation":generation,"sha256":entries}))?;
    let key = PKey::generate_ed25519()?;
    let public_key = String::from_utf8(key.public_key_to_pem()?)?;
    let signature = Signer::new_without_digest(&key)?.sign_oneshot_to_vec(&manifest)?;
    drop(key);
    let mut archive =
        zip::ZipWriter::new_append(OpenOptions::new().read(true).write(true).open(path)?)?;
    let options: FileOptions<()> =
        FileOptions::default().compression_method(zip::CompressionMethod::DEFLATE);
    for (name, bytes) in [
        ("manifest.json", manifest.as_slice()),
        ("manifest.sig", signature.as_slice()),
        ("public-key.pem", public_key.as_bytes()),
    ] {
        archive.start_file(format!("execution-integrity/{name}"), options)?;
        archive.write_all(bytes)?;
    }
    archive.finish()?;
    Ok(ArchiveSignature {
        manifest_sha256: hex::encode(Sha256::digest(&manifest)),
        public_key,
    })
}

/// Execution archives retain only public identity-provider configuration. Do not
/// call the import-time realm rewriter here: it injects deployment credentials.
pub fn public_realm_configuration(realm: &serde_json::Value) -> serde_json::Value {
    fn strip(value: &mut serde_json::Value) {
        match value {
            serde_json::Value::Object(object) => {
                object.retain(|name, _| {
                    let key = name.to_ascii_lowercase().replace(['-', '_', '.'], "");
                    !key.contains("secret")
                        && !key.contains("password")
                        && !key.contains("privatekey")
                        && !key.contains("credential")
                        && !matches!(
                            key.as_str(),
                            "accesstoken"
                                | "refreshtoken"
                                | "registrationaccesstoken"
                                | "initialaccesstoken"
                                | "idtoken"
                        )
                });
                for child in object.values_mut() {
                    strip(child);
                }
            }
            serde_json::Value::Array(items) => {
                for child in items {
                    strip(child);
                }
            }
            _ => {}
        }
    }
    let mut public = realm.clone();
    if let Some(object) = public.as_object_mut() {
        // Provider components include generated signing-key material. User data
        // is exported independently under the voter-secret permission policy.
        object.remove("components");
        object.remove("users");
        object.remove("federatedUsers");
    }
    strip(&mut public);
    public
}

#[cfg(test)]
mod tests {
    use super::*;
    use openssl::sign::Verifier;
    #[test]
    fn execution_realm_export_excludes_deployment_credentials_and_private_keys() {
        let original = json!({"realm":"event", "enabled":true,
            "clients":[{"clientId":"service", "secret":"DO-NOT-EXPORT", "publicClient":false,
                "attributes":{"jwt.credential":"DO-NOT-EXPORT", "public-key":"public"}}],
            "identityProviders":[{"alias":"external", "config":{"clientId":"id", "clientSecret":"DO-NOT-EXPORT", "authorizationUrl":"https://idp.example/authorize"}}],
            "smtpServer":{"password":"DO-NOT-EXPORT", "host":"mail.example"},
            "components":{"key-provider":[{"config":{"privateKey":["DO-NOT-EXPORT"]}}]},
            "users":[{"username":"sensitive-register-row","credentials":[{"value":"DO-NOT-EXPORT"}]}],
            "registrationAccessToken":"DO-NOT-EXPORT"});
        let public = public_realm_configuration(&original);
        let encoded = serde_json::to_string(&public).unwrap();
        assert!(!encoded.contains("DO-NOT-EXPORT"));
        assert!(!encoded.contains("sensitive-register-row"));
        assert_eq!(public["clients"][0]["clientId"], "service");
        assert_eq!(
            public["identityProviders"][0]["config"]["authorizationUrl"],
            "https://idp.example/authorize"
        );
        assert_eq!(public["clients"][0]["attributes"]["public-key"], "public");
        assert_eq!(original["clients"][0]["secret"], "DO-NOT-EXPORT");
    }
    #[test]
    fn archive_signature_covers_independent_file_digest_and_rejects_changed_manifest() {
        let file = tempfile::NamedTempFile::new().unwrap();
        let mut zip = zip::ZipWriter::new(file.reopen().unwrap());
        zip.start_file("ballots.txt", FileOptions::<()>::default())
            .unwrap();
        zip.write_all(b"abc").unwrap();
        zip.finish().unwrap();
        let signed = sign(file.path(), "tenant", "event", "document", 17, 4).unwrap();
        let mut zip = zip::ZipArchive::new(file.reopen().unwrap()).unwrap();
        let mut manifest = Vec::new();
        zip.by_name("execution-integrity/manifest.json")
            .unwrap()
            .read_to_end(&mut manifest)
            .unwrap();
        let parsed: serde_json::Value = serde_json::from_slice(&manifest).unwrap();
        assert_eq!(
            parsed["sha256"]["ballots.txt"],
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(parsed["source_revision"], 17);
        let mut signature = Vec::new();
        zip.by_name("execution-integrity/manifest.sig")
            .unwrap()
            .read_to_end(&mut signature)
            .unwrap();
        let key = PKey::public_key_from_pem(signed.public_key.as_bytes()).unwrap();
        assert!(Verifier::new_without_digest(&key)
            .unwrap()
            .verify_oneshot(&signature, &manifest)
            .unwrap());
        manifest[0] = b'!';
        assert!(!Verifier::new_without_digest(&key)
            .unwrap()
            .verify_oneshot(&signature, &manifest)
            .unwrap());
        assert!(zip.by_name("execution-integrity/private-key.pem").is_err());
    }
}
