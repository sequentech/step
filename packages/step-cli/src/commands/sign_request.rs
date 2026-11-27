// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Signs a signing request with the signer's certificate, as the admin
//! portal's signing dialog does: the approval signs the request's canonical
//! payload, which must hash to its `payload_sha256` and be about the request.

use crate::{types::hasura_types::*, utils::read_config::read_config};
use anyhow::{anyhow, bail, ensure, Context, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use clap::Args;
use graphql_client::{GraphQLQuery, Response};
use openssl::{
    hash::MessageDigest,
    nid::Nid,
    pkcs12::Pkcs12,
    pkey::{Id, PKey, Private},
    rsa::Padding,
    sign::Signer,
    x509::{X509VerifyResult, X509},
};
use sequent_core::{
    services::jwt::decode_jwt,
    signing::{
        canonical_json, DocumentKind, SignatureAlgorithm, SigningAction, SigningRequestStatus,
        SIGNING_DOMAIN,
    },
    types::permissions::Permissions,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use strum::IntoEnumIterator;

/// The environment variable that holds the .p12 password; without it the
/// password is asked for on the terminal.
pub const P12_PASSWORD_ENV: &str = "STEP_SIGNING_P12_PASSWORD";

const HASURA_ROLE_HEADER: &str = "x-hasura-role";

#[derive(Args)]
#[command(
    about = "Sign a signing request with your certificate",
    long_about = "Sign a signing request with your certificate (.p12/.pfx). The password is \
                  read from STEP_SIGNING_P12_PASSWORD or asked for on the terminal. Requests \
                  that also sign a PDF or EML document are not supported yet."
)]
pub struct SignRequest {
    /// The signing request id
    #[arg(long)]
    id: String,

    /// The certificate file (.p12/.pfx) with your private key
    #[arg(long, required_unless_present = "verify_only")]
    p12: Option<PathBuf>,

    /// Fetch and check the request (and open the certificate, if given) without signing it
    #[arg(long)]
    verify_only: bool,
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/signing_get_request.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]
pub struct SigningGetRequest;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/signing_approve.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]
pub struct SigningApprove;

/// The fields of `panel.request` this command reads.
#[derive(Debug, Clone, Deserialize)]
pub struct PanelRequest {
    pub id: String,
    pub tenant_id: String,
    pub election_event_id: String,
    #[serde(default)]
    pub election_id: Option<String>,
    #[serde(default)]
    pub area_id: Option<String>,
    pub action: SigningAction,
    pub status: SigningRequestStatus,
    pub code: String,
    pub canonical_payload: String,
    pub payload_sha256: String,
    pub required: i64,
    #[serde(default)]
    pub expires_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Panel {
    pub request: PanelRequest,
    pub count: i64,
}

/// The calls of this command, grouped by the roles their Hasura action allows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SigningOperation {
    /// signingGetRequest: admin-user, every sign-<action> and signing-requests-read.
    GetRequest,
    /// signingApprove: admin-user and the sign-<action> permissions.
    Sign,
}

/// The role to name, as the admin portal picks it: one the operation's action
/// allows and the user holds, the request's own sign permission first.
pub fn signing_role(
    allowed_roles: &[String],
    operation: SigningOperation,
    action: Option<SigningAction>,
) -> Option<Permissions> {
    let own = action.map(|action| action.sign_permission());
    let mut candidates: Vec<Permissions> = own.into_iter().collect();
    if operation == SigningOperation::GetRequest {
        candidates.push(Permissions::SIGNING_REQUESTS_READ);
    }
    candidates.push(Permissions::ADMIN_USER);
    candidates.extend(SigningAction::iter().map(|action| action.sign_permission()));
    candidates
        .into_iter()
        .find(|role| allowed_roles.contains(&role.to_string()))
}

/// The bytes the approval signs: the canonical payload exactly as the server
/// sent it, once it hashes to `payload_sha256`, is in canonical form and is
/// about this request.
pub fn payload_to_sign(request: &PanelRequest) -> Result<Vec<u8>> {
    let bytes = request.canonical_payload.as_bytes();
    ensure!(
        hex::encode(Sha256::digest(bytes)) == request.payload_sha256.to_lowercase(),
        "the canonical payload does not hash to payload_sha256"
    );
    let payload: Value = serde_json::from_str(&request.canonical_payload)
        .context("the canonical payload is not JSON")?;
    ensure!(
        payload.is_object(),
        "the canonical payload is not an object"
    );
    ensure!(
        payload["domain"] == SIGNING_DOMAIN
            && canonical_json(&payload).ok().as_deref() == Some(request.canonical_payload.as_str()),
        "the payload is not in canonical form"
    );
    let expected = [
        ("request_id", Some(request.id.clone())),
        ("code", Some(request.code.clone())),
        ("action", Some(request.action.to_string())),
        ("tenant_id", Some(request.tenant_id.clone())),
        ("election_event_id", Some(request.election_event_id.clone())),
        ("election_id", request.election_id.clone()),
        ("area_id", request.area_id.clone()),
    ];
    for (field, value) in expected {
        ensure!(
            payload[field].as_str() == value.as_deref(),
            "the payload's {field} is not the request's"
        );
    }
    ensure!(payload["subject"].is_object(), "the payload has no subject");
    Ok(bytes.to_vec())
}

/// Refuses what this command can't sign yet: a request that no longer waits,
/// or one whose approval also signs a document.
pub fn check_signable(request: &PanelRequest) -> Result<()> {
    ensure!(
        request.status == SigningRequestStatus::Waiting,
        "the request is {}, not waiting for signatures",
        request.status
    );
    match request.action.document() {
        DocumentKind::NoDocument => Ok(()),
        kind => bail!(
            "signing a {} request ({kind} document) is not supported yet by step-cli; \
             sign it in the admin portal",
            request.action
        ),
    }
}

/// A private key and its certificate chain, leaf first.
pub struct Certificate {
    pub key: PKey<Private>,
    pub algorithm: SignatureAlgorithm,
    pub chain: Vec<X509>,
}

impl Certificate {
    pub fn open_p12(bytes: &[u8], password: &str) -> Result<Self> {
        let parsed = Pkcs12::from_der(bytes)
            .context("the file is not a PKCS#12 (.p12/.pfx) file")?
            .parse2(password)
            .map_err(|_| {
                anyhow!("the .p12 could not be opened: wrong password or unsupported encryption")
            })?;
        let key = parsed
            .pkey
            .ok_or_else(|| anyhow!("the .p12 has no private key"))?;
        let leaf = parsed
            .cert
            .ok_or_else(|| anyhow!("the .p12 has no certificate"))?;
        ensure!(
            leaf.public_key()?.public_eq(&key),
            "the .p12's certificate does not match its private key"
        );
        let others = parsed
            .ca
            .map(|stack| stack.into_iter().collect())
            .unwrap_or_default();
        Ok(Certificate {
            algorithm: algorithm_of(&key)?,
            chain: order_chain(leaf, others),
            key,
        })
    }

    pub fn chain_pem(&self) -> Result<Vec<String>> {
        self.chain
            .iter()
            .map(|certificate| Ok(String::from_utf8(certificate.to_pem()?)?))
            .collect()
    }

    pub fn subject(&self) -> String {
        self.chain[0]
            .subject_name()
            .entries()
            .filter_map(|entry| {
                let value = entry.data().as_utf8().ok()?;
                Some(format!(
                    "{}={}",
                    entry.object().nid().short_name().ok()?,
                    value
                ))
            })
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// Signs `data` as the platform verifies it: PKCS#1 v1.5 with SHA-256, or
    /// ECDSA P-256 with SHA-256 as a DER signature.
    pub fn sign(&self, data: &[u8]) -> Result<Vec<u8>> {
        let mut signer = Signer::new(MessageDigest::sha256(), &self.key)?;
        if self.algorithm == SignatureAlgorithm::RsaPkcs1Sha256 {
            signer.set_rsa_padding(Padding::PKCS1)?;
        }
        signer.update(data)?;
        Ok(signer.sign_to_vec()?)
    }
}

fn algorithm_of(key: &PKey<Private>) -> Result<SignatureAlgorithm> {
    match key.id() {
        Id::RSA => Ok(SignatureAlgorithm::RsaPkcs1Sha256),
        Id::EC if key.ec_key()?.group().curve_name() == Some(Nid::X9_62_PRIME256V1) => {
            Ok(SignatureAlgorithm::EcdsaP256Sha256)
        }
        _ => bail!("the key is neither RSA nor ECDSA P-256"),
    }
}

/// Whether `issuer` names and signed `certificate`.
fn issued_by(certificate: &X509, issuer: &X509) -> bool {
    issuer.issued(certificate) == X509VerifyResult::OK
        && issuer
            .public_key()
            .and_then(|key| certificate.verify(&key))
            .unwrap_or(false)
}

/// The leaf and then each issuer found among `others`.
fn order_chain(leaf: X509, mut others: Vec<X509>) -> Vec<X509> {
    let mut chain = vec![leaf];
    loop {
        let last = &chain[chain.len() - 1];
        if issued_by(last, last) {
            break;
        }
        match others
            .iter()
            .position(|candidate| issued_by(last, candidate))
        {
            Some(index) => {
                let issuer = others.remove(index);
                chain.push(issuer);
            }
            None => break,
        }
    }
    chain
}

fn read_password() -> Result<String> {
    if let Ok(password) = std::env::var(P12_PASSWORD_ENV) {
        return Ok(password);
    }
    prompt_password("Certificate password: ")
}

/// Reads a line from the terminal without echoing it.
fn prompt_password(prompt: &str) -> Result<String> {
    use std::io::{BufRead, Write};
    use std::os::fd::AsRawFd;
    let tty = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/tty")
        .with_context(|| format!("no terminal to ask for the password; set {P12_PASSWORD_ENV}"))?;
    let fd = tty.as_raw_fd();
    let mut original = std::mem::MaybeUninit::<libc::termios>::uninit();
    // SAFETY: fd is an open terminal and `original` is written by tcgetattr.
    ensure!(
        unsafe { libc::tcgetattr(fd, original.as_mut_ptr()) } == 0,
        "could not read the terminal settings"
    );
    let original = unsafe { original.assume_init() };
    let mut silent = original;
    silent.c_lflag &= !libc::ECHO;
    (&tty).write_all(prompt.as_bytes())?;
    (&tty).flush()?;
    unsafe { libc::tcsetattr(fd, libc::TCSANOW, &silent) };
    let mut line = String::new();
    let read = std::io::BufReader::new(&tty).read_line(&mut line);
    unsafe { libc::tcsetattr(fd, libc::TCSANOW, &original) };
    (&tty).write_all(b"\n")?;
    read?;
    Ok(line.trim_end_matches(['\r', '\n']).to_owned())
}

fn graphql<Q: Serialize, R: serde::de::DeserializeOwned>(
    query: &Q,
    role: Option<Permissions>,
) -> Result<R> {
    let config = read_config().map_err(|error| anyhow!("{error}"))?;
    let mut request = reqwest::blocking::Client::new()
        .post(&config.endpoint_url)
        .bearer_auth(&config.auth_token);
    if let Some(role) = role {
        request = request.header(HASURA_ROLE_HEADER, role.to_string());
    }
    let response = request.json(query).send()?;
    let status = response.status();
    if !status.is_success() {
        bail!("HTTP Status: {status}\nError Message: {}", response.text()?);
    }
    let body: Response<R> = response.json()?;
    if let Some(errors) = body.errors.filter(|errors| !errors.is_empty()) {
        let messages: Vec<String> = errors
            .into_iter()
            .map(|error| {
                match error
                    .extensions
                    .as_ref()
                    .and_then(|extensions| extensions.get("code"))
                    .and_then(Value::as_str)
                {
                    Some(code) => format!("{} ({code})", error.message),
                    None => error.message,
                }
            })
            .collect();
        bail!("{}", messages.join(", "));
    }
    body.data.ok_or_else(|| anyhow!("the answer has no data"))
}

fn allowed_roles() -> Vec<String> {
    read_config()
        .ok()
        .and_then(|config| decode_jwt(&config.auth_token).ok())
        .map(|claims| claims.hasura_claims.allowed_roles)
        .unwrap_or_default()
}

impl SignRequest {
    pub fn run(&self) -> Result<()> {
        let roles = allowed_roles();
        let data: signing_get_request::ResponseData = graphql(
            &SigningGetRequest::build_query(signing_get_request::Variables {
                request_id: self.id.clone(),
            }),
            signing_role(&roles, SigningOperation::GetRequest, None),
        )?;
        let panel: Panel = serde_json::from_value(data.signing_get_request.panel)
            .context("the signing request is malformed")?;
        let request = &panel.request;
        ensure!(
            request.id == self.id,
            "the server answered another request ({})",
            request.id
        );
        let payload = payload_to_sign(request)?;
        println!(
            "Request {} ({}), code {}: {} of {} signatures{}",
            request.id,
            request.action,
            request.code,
            panel.count,
            request.required,
            request
                .expires_at
                .as_deref()
                .map(|at| format!(", expires at {at}"))
                .unwrap_or_default()
        );
        println!("Signed payload: {}", request.canonical_payload);
        check_signable(request)?;

        let certificate = match &self.p12 {
            Some(path) => {
                let bytes = std::fs::read(path)
                    .with_context(|| format!("could not read {}", path.display()))?;
                let certificate = Certificate::open_p12(&bytes, &read_password()?)?;
                println!(
                    "Certificate: {} ({})",
                    certificate.subject(),
                    certificate.algorithm
                );
                Some(certificate)
            }
            None => None,
        };
        if self.verify_only {
            println!("The request checks out; not signed (--verify-only).");
            return Ok(());
        }
        let certificate = certificate.ok_or_else(|| anyhow!("--p12 is required to sign"))?;
        let signature = certificate.sign(&payload)?;
        let approved: signing_approve::ResponseData = graphql(
            &SigningApprove::build_query(signing_approve::Variables {
                request_id: request.id.clone(),
                chain_pem: certificate.chain_pem()?,
                algorithm: certificate.algorithm.to_string(),
                payload_signature_b64: BASE64.encode(signature),
            }),
            signing_role(&roles, SigningOperation::Sign, Some(request.action)),
        )?;
        let answer = approved.signing_approve;
        println!(
            "Success! Signed request {}: {} ({} of {} signatures)",
            request.id, answer.status, answer.count, answer.required
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openssl::{
        asn1::Asn1Time,
        bn::BigNum,
        ec::{EcGroup, EcKey},
        rsa::Rsa,
        x509::{X509Builder, X509NameBuilder},
    };
    use serde_json::json;
    use windmill::services::signing::certificates::verify_signature;

    const REQUEST_ID: &str = "4e0c0d6e-8f35-4c3b-9a43-0b8f1d1c2a11";

    fn request_with(payload: Value) -> PanelRequest {
        let canonical = canonical_json(&payload).unwrap();
        PanelRequest {
            id: REQUEST_ID.into(),
            tenant_id: "t1".into(),
            election_event_id: "e1".into(),
            election_id: None,
            area_id: None,
            action: SigningAction::ConfirmCustodyProof,
            status: SigningRequestStatus::Waiting,
            code: "K7Q-2M".into(),
            payload_sha256: hex::encode(Sha256::digest(canonical.as_bytes())),
            canonical_payload: canonical,
            required: 1,
            expires_at: None,
        }
    }

    fn payload() -> Value {
        json!({
            "domain": SIGNING_DOMAIN, "request_id": REQUEST_ID, "code": "K7Q-2M",
            "action": "key-ceremony", "tenant_id": "t1", "election_event_id": "e1",
            "election_id": null, "area_id": null,
            "subject": {"custody_proof_sha256": "ab", "trustee_id": "tr1"}
        })
    }

    #[test]
    fn signs_the_canonical_payload_bytes_as_sent() {
        let request = request_with(payload());
        assert_eq!(
            payload_to_sign(&request).unwrap(),
            request.canonical_payload.as_bytes()
        );
    }

    #[test]
    fn refuses_a_payload_that_does_not_hash_to_payload_sha256() {
        let mut request = request_with(payload());
        request.payload_sha256 = hex::encode(Sha256::digest(b"other"));
        assert!(payload_to_sign(&request)
            .unwrap_err()
            .to_string()
            .contains("does not hash"));
    }

    #[test]
    fn refuses_a_payload_about_another_request() {
        let mut other = payload();
        other["code"] = json!("ZZZ-99");
        let error = payload_to_sign(&request_with(other)).unwrap_err();
        assert!(error.to_string().contains("code"), "{error}");
    }

    #[test]
    fn refuses_a_payload_of_another_domain_or_not_canonical() {
        let mut other = payload();
        other["domain"] = json!("other/v1");
        assert!(payload_to_sign(&request_with(other)).is_err());

        let mut request = request_with(payload());
        request.canonical_payload = serde_json::to_string_pretty(&payload()).unwrap();
        request.payload_sha256 = hex::encode(Sha256::digest(request.canonical_payload.as_bytes()));
        assert!(payload_to_sign(&request)
            .unwrap_err()
            .to_string()
            .contains("canonical form"));
    }

    #[test]
    fn document_requests_and_closed_requests_are_refused() {
        let mut request = request_with(payload());
        assert!(check_signable(&request).is_ok());
        request.action = SigningAction::GenerateReports;
        assert!(check_signable(&request)
            .unwrap_err()
            .to_string()
            .contains("not supported yet"));
        request.action = SigningAction::TransmitResults;
        assert!(check_signable(&request).is_err());
        request.action = SigningAction::OpenVoting;
        request.status = SigningRequestStatus::Completed;
        assert!(check_signable(&request)
            .unwrap_err()
            .to_string()
            .contains("not waiting"));
    }

    #[test]
    fn reads_a_panel_answer() {
        let mut value = json!({
            "request": {
                "id": REQUEST_ID, "tenant_id": "t1", "election_event_id": "e1",
                "action": "tally-key", "status": "waiting", "code": "K7Q-2M",
                "canonical_payload": "{}", "payload_sha256": "00", "required": 2,
                "expires_at": null, "requested_by": "u1"
            },
            "count": 1, "signers": [], "rule": {"requester_signing": "allowed"}
        });
        let panel: Panel = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(panel.request.action, SigningAction::ContributeCustodyProof);
        assert_eq!(panel.count, 1);
        value["request"]["action"] = json!("unknown-action");
        assert!(serde_json::from_value::<Panel>(value).is_err());
    }

    #[test]
    fn names_the_role_the_admin_portal_would() {
        let roles = |names: &[Permissions]| -> Vec<String> {
            names.iter().map(ToString::to_string).collect()
        };
        let trustee = roles(&[
            Permissions::TRUSTEE_CEREMONY,
            Permissions::SIGN_KEY_CEREMONY,
        ]);
        assert_eq!(
            signing_role(&trustee, SigningOperation::GetRequest, None),
            Some(Permissions::SIGN_KEY_CEREMONY)
        );
        assert_eq!(
            signing_role(
                &trustee,
                SigningOperation::Sign,
                Some(SigningAction::ConfirmCustodyProof)
            ),
            Some(Permissions::SIGN_KEY_CEREMONY)
        );
        let reader = roles(&[Permissions::SIGNING_REQUESTS_READ, Permissions::ADMIN_USER]);
        assert_eq!(
            signing_role(&reader, SigningOperation::GetRequest, None),
            Some(Permissions::SIGNING_REQUESTS_READ)
        );
        assert_eq!(
            signing_role(
                &reader,
                SigningOperation::Sign,
                Some(SigningAction::OpenVoting)
            ),
            Some(Permissions::ADMIN_USER)
        );
        assert_eq!(
            signing_role(
                &roles(&[Permissions::TRUSTEE_CEREMONY]),
                SigningOperation::Sign,
                None
            ),
            None
        );
    }

    fn self_signed(key: &PKey<Private>, digest: MessageDigest) -> X509 {
        let mut name = X509NameBuilder::new().unwrap();
        name.append_entry_by_text("CN", "Test signer").unwrap();
        let name = name.build();
        let mut builder = X509Builder::new().unwrap();
        builder.set_version(2).unwrap();
        builder
            .set_serial_number(&BigNum::from_u32(1).unwrap().to_asn1_integer().unwrap())
            .unwrap();
        builder.set_subject_name(&name).unwrap();
        builder.set_issuer_name(&name).unwrap();
        builder.set_pubkey(key).unwrap();
        builder
            .set_not_before(&Asn1Time::days_from_now(0).unwrap())
            .unwrap();
        builder
            .set_not_after(&Asn1Time::days_from_now(1).unwrap())
            .unwrap();
        builder.sign(key, digest).unwrap();
        builder.build()
    }

    fn p12_of(key: &PKey<Private>, certificate: &X509, password: &str) -> Vec<u8> {
        let mut builder = Pkcs12::builder();
        builder.name("signer").pkey(key).cert(certificate);
        builder.build2(password).unwrap().to_der().unwrap()
    }

    fn signs_as_the_platform_verifies(key: PKey<Private>, expected: SignatureAlgorithm) {
        let certificate = self_signed(&key, MessageDigest::sha256());
        let p12 = p12_of(&key, &certificate, "secret");
        assert!(Certificate::open_p12(&p12, "wrong").is_err());
        let opened = Certificate::open_p12(&p12, "secret").unwrap();
        assert_eq!(opened.algorithm, expected);
        assert_eq!(opened.chain.len(), 1);
        assert!(opened.chain_pem().unwrap()[0].starts_with("-----BEGIN CERTIFICATE-----"));
        assert!(opened.subject().contains("Test signer"));

        let request = request_with(payload());
        let data = payload_to_sign(&request).unwrap();
        let signature = opened.sign(&data).unwrap();
        assert!(verify_signature(&certificate, expected, &data, &signature).unwrap());
        assert!(!verify_signature(&certificate, expected, b"other bytes", &signature).unwrap());
    }

    #[test]
    fn an_rsa_certificate_signs_with_pkcs1_sha256() {
        let key = PKey::from_rsa(Rsa::generate(2048).unwrap()).unwrap();
        signs_as_the_platform_verifies(key, SignatureAlgorithm::RsaPkcs1Sha256);
    }

    #[test]
    fn a_p256_certificate_signs_with_der_ecdsa_sha256() {
        let group = EcGroup::from_curve_name(Nid::X9_62_PRIME256V1).unwrap();
        let key = PKey::from_ec_key(EcKey::generate(&group).unwrap()).unwrap();
        signs_as_the_platform_verifies(key, SignatureAlgorithm::EcdsaP256Sha256);
    }

    #[test]
    fn another_curve_is_refused() {
        let group = EcGroup::from_curve_name(Nid::SECP384R1).unwrap();
        let key = PKey::from_ec_key(EcKey::generate(&group).unwrap()).unwrap();
        let certificate = self_signed(&key, MessageDigest::sha384());
        let p12 = p12_of(&key, &certificate, "secret");
        assert!(Certificate::open_p12(&p12, "secret").is_err());
    }

    #[test]
    fn the_chain_runs_from_the_leaf_to_its_issuers() {
        let root_key = PKey::from_rsa(Rsa::generate(2048).unwrap()).unwrap();
        let root = self_signed(&root_key, MessageDigest::sha256());
        let leaf_key = PKey::from_rsa(Rsa::generate(2048).unwrap()).unwrap();
        let mut name = X509NameBuilder::new().unwrap();
        name.append_entry_by_text("CN", "Leaf").unwrap();
        let name = name.build();
        let mut builder = X509Builder::new().unwrap();
        builder.set_version(2).unwrap();
        builder
            .set_serial_number(&BigNum::from_u32(2).unwrap().to_asn1_integer().unwrap())
            .unwrap();
        builder.set_subject_name(&name).unwrap();
        builder.set_issuer_name(root.subject_name()).unwrap();
        builder.set_pubkey(&leaf_key).unwrap();
        builder
            .set_not_before(&Asn1Time::days_from_now(0).unwrap())
            .unwrap();
        builder
            .set_not_after(&Asn1Time::days_from_now(1).unwrap())
            .unwrap();
        builder.sign(&root_key, MessageDigest::sha256()).unwrap();
        let leaf = builder.build();
        let unrelated = self_signed(
            &PKey::from_rsa(Rsa::generate(2048).unwrap()).unwrap(),
            MessageDigest::sha256(),
        );
        let chain = order_chain(leaf.clone(), vec![unrelated, root.clone()]);
        let ders: Vec<Vec<u8>> = chain.iter().map(|c| c.to_der().unwrap()).collect();
        assert_eq!(ders, vec![leaf.to_der().unwrap(), root.to_der().unwrap()]);
    }
}
