// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The PKCS#11 provider: a YubiHSM 2 through Yubico's `yubihsm_pkcs11`
//! module, or SoftHSM for tests and demos without hardware.
//!
//! The configured backend must match the module that answers: a SoftHSM
//! module is refused when `yubihsm` is configured and the reverse, so one
//! can never stand in for the other without anyone noticing. Only a Yubico
//! token is reported as hardware.

use std::path::Path;
use std::sync::Mutex;

use cryptoki::context::{CInitializeArgs, CInitializeFlags, Pkcs11};
use cryptoki::mechanism::aead::GcmParams;
use cryptoki::mechanism::eddsa::{EddsaParams, EddsaSignatureScheme};
use cryptoki::mechanism::elliptic_curve::{EcKdf, Ecdh1DeriveParams};
use cryptoki::mechanism::Mechanism;
use cryptoki::object::{Attribute, AttributeType, KeyType, ObjectClass, ObjectHandle};
use cryptoki::session::{Session, UserType};
use cryptoki::slot::Slot;
use cryptoki::types::AuthPin;
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

use crate::formats::{unwrap_ec_point, ED25519_PARAMS, P256_PARAMS};
use crate::provider::{
    Algorithm, KeyAttributes, Provider, ProviderError, ProviderInfo, ProviderResult,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pkcs11Backend {
    SoftHsm,
    YubiHsm,
}

impl Pkcs11Backend {
    pub fn name(self) -> &'static str {
        match self {
            Pkcs11Backend::SoftHsm => "softhsm",
            Pkcs11Backend::YubiHsm => "yubihsm",
        }
    }

    fn matches(self, manufacturer: &str, description: &str) -> bool {
        let text = format!("{manufacturer} {description}").to_lowercase();
        match self {
            Pkcs11Backend::SoftHsm => text.contains("softhsm"),
            Pkcs11Backend::YubiHsm => text.contains("yubico") || text.contains("yubihsm"),
        }
    }
}

const GCM_IV_BYTES: usize = 12;
const GCM_TAG_BITS: u64 = 128;
const MAX_IDLE_SESSIONS: usize = 8;

pub struct Pkcs11Provider {
    backend: Pkcs11Backend,
    context: Pkcs11,
    slot: Slot,
    info: ProviderInfo,
    // Keeps the application logged in; every other session shares the login.
    _login: Mutex<Session>,
    idle: Mutex<Vec<Session>>,
}

fn device<E: std::fmt::Display>(error: E) -> ProviderError {
    ProviderError::Device(error.to_string())
}

impl Pkcs11Provider {
    /// `pin` is the token's user PIN; on a YubiHSM 2 it is the
    /// authentication key's id (4 hex digits) followed by its password.
    pub fn open(
        backend: Pkcs11Backend,
        module: &Path,
        token_label: &str,
        pin: Zeroizing<String>,
    ) -> ProviderResult<Self> {
        let context = Pkcs11::new(module).map_err(device)?;
        context
            .initialize(CInitializeArgs::new(CInitializeFlags::OS_LOCKING_OK))
            .map_err(device)?;
        let library = context.get_library_info().map_err(device)?;
        if !backend.matches(library.manufacturer_id(), library.library_description()) {
            return Err(ProviderError::Device(format!(
                "backend '{}' is configured but the module is '{} {}'",
                backend.name(),
                library.manufacturer_id().trim(),
                library.library_description().trim()
            )));
        }
        let slot = context
            .get_slots_with_token()
            .map_err(device)?
            .into_iter()
            .find(|slot| {
                context
                    .get_token_info(*slot)
                    .map(|info| info.label().trim() == token_label)
                    .unwrap_or(false)
            })
            .ok_or_else(|| {
                ProviderError::Device(format!("no token is labelled '{token_label}'"))
            })?;
        let token = context.get_token_info(slot).map_err(device)?;
        let hardware = backend == Pkcs11Backend::YubiHsm
            && token.manufacturer_id().to_lowercase().contains("yubico");
        let info = ProviderInfo {
            backend: backend.name().to_string(),
            library: library.library_description().trim().to_string(),
            library_version: library.library_version().to_string(),
            token_model: token.model().trim().to_string(),
            token_manufacturer: token.manufacturer_id().trim().to_string(),
            firmware_version: token.firmware_version().to_string(),
            serial_sha256_prefix: hex::encode(&Sha256::digest(token.serial_number().trim())[..4]),
            hardware,
        };
        let login = context.open_rw_session(slot).map_err(device)?;
        let pin = AuthPin::new(pin.as_str().into());
        login.login(UserType::User, Some(&pin)).map_err(device)?;
        Ok(Pkcs11Provider {
            backend,
            context,
            slot,
            info,
            _login: Mutex::new(login),
            idle: Mutex::new(Vec::new()),
        })
    }

    pub fn backend(&self) -> Pkcs11Backend {
        self.backend
    }

    fn with_session<T>(
        &self,
        run: impl FnOnce(&Session) -> ProviderResult<T>,
    ) -> ProviderResult<T> {
        let session = match self.idle.lock().map_err(device)?.pop() {
            Some(session) => session,
            None => self.context.open_rw_session(self.slot).map_err(device)?,
        };
        let result = run(&session);
        if let Ok(mut idle) = self.idle.lock() {
            if idle.len() < MAX_IDLE_SESSIONS {
                idle.push(session);
            }
        }
        result
    }

    fn find(
        &self,
        session: &Session,
        class: ObjectClass,
        label: &str,
    ) -> ProviderResult<ObjectHandle> {
        session
            .find_objects(&[
                Attribute::Class(class),
                Attribute::Label(label.as_bytes().to_vec()),
            ])
            .map_err(device)?
            .into_iter()
            .next()
            .ok_or_else(|| ProviderError::NoSuchKey(label.to_string()))
    }

    fn private_class(algorithm: Algorithm) -> ObjectClass {
        if algorithm.is_asymmetric() {
            ObjectClass::PRIVATE_KEY
        } else {
            ObjectClass::SECRET_KEY
        }
    }

    fn exists(&self, session: &Session, label: &str) -> ProviderResult<bool> {
        Ok(!session
            .find_objects(&[Attribute::Label(label.as_bytes().to_vec())])
            .map_err(device)?
            .is_empty())
    }
}

fn key_id(label: &str) -> Vec<u8> {
    Sha256::digest(label.as_bytes())[..16].to_vec()
}

impl Provider for Pkcs11Provider {
    fn info(&self) -> ProviderInfo {
        self.info.clone()
    }

    fn generate(&self, label: &str, algorithm: Algorithm) -> ProviderResult<Vec<u8>> {
        self.with_session(|session| {
            if self.exists(session, label)? {
                return Err(ProviderError::Exists(label.to_string()));
            }
            let id = key_id(label);
            let common_private = vec![
                Attribute::Token(true),
                Attribute::Private(true),
                Attribute::Sensitive(true),
                Attribute::Extractable(false),
                Attribute::Id(id.clone()),
                Attribute::Label(label.as_bytes().to_vec()),
            ];
            let common_public = vec![
                Attribute::Token(true),
                Attribute::Private(false),
                Attribute::Id(id.clone()),
                Attribute::Label(label.as_bytes().to_vec()),
            ];
            match algorithm {
                Algorithm::Ed25519 | Algorithm::EcdsaP256 | Algorithm::EcdhP256 => {
                    let (mechanism, params, signs) = match algorithm {
                        Algorithm::Ed25519 => (
                            Mechanism::EccEdwardsKeyPairGen,
                            ED25519_PARAMS.to_vec(),
                            true,
                        ),
                        Algorithm::EcdsaP256 => {
                            (Mechanism::EccKeyPairGen, P256_PARAMS.to_vec(), true)
                        }
                        _ => (Mechanism::EccKeyPairGen, P256_PARAMS.to_vec(), false),
                    };
                    let mut public = common_public;
                    public.push(Attribute::EcParams(params));
                    public.push(Attribute::Verify(signs));
                    let mut private = common_private;
                    private.push(Attribute::Sign(signs));
                    private.push(Attribute::Derive(!signs));
                    let (public_handle, _) = session
                        .generate_key_pair(&mechanism, &public, &private)
                        .map_err(device)?;
                    read_point(session, public_handle, algorithm)
                }
                Algorithm::HmacSha256 => {
                    let mut template = common_private;
                    template.push(Attribute::Class(ObjectClass::SECRET_KEY));
                    template.push(Attribute::KeyType(KeyType::GENERIC_SECRET));
                    template.push(Attribute::ValueLen(32.into()));
                    template.push(Attribute::Sign(true));
                    template.push(Attribute::Verify(true));
                    session
                        .generate_key(&Mechanism::GenericSecretKeyGen, &template)
                        .map_err(device)?;
                    Ok(Vec::new())
                }
                Algorithm::Aes256Gcm => {
                    let mut template = common_private;
                    template.push(Attribute::Class(ObjectClass::SECRET_KEY));
                    template.push(Attribute::KeyType(KeyType::AES));
                    template.push(Attribute::ValueLen(32.into()));
                    template.push(Attribute::Encrypt(true));
                    template.push(Attribute::Decrypt(true));
                    session
                        .generate_key(&Mechanism::AesKeyGen, &template)
                        .map_err(device)?;
                    Ok(Vec::new())
                }
            }
        })
    }

    fn public_key(&self, label: &str, algorithm: Algorithm) -> ProviderResult<Vec<u8>> {
        if !algorithm.is_asymmetric() {
            return Err(ProviderError::Unsupported(format!(
                "a public key for {algorithm:?}"
            )));
        }
        self.with_session(|session| {
            let handle = self.find(session, ObjectClass::PUBLIC_KEY, label)?;
            read_point(session, handle, algorithm)
        })
    }

    fn sign(&self, label: &str, algorithm: Algorithm, message: &[u8]) -> ProviderResult<Vec<u8>> {
        self.with_session(|session| {
            let handle = self.find(session, Self::private_class(algorithm), label)?;
            match algorithm {
                Algorithm::Ed25519 => session
                    .sign(
                        &Mechanism::Eddsa(EddsaParams::new(EddsaSignatureScheme::Pure)),
                        handle,
                        message,
                    )
                    .map_err(device),
                Algorithm::EcdsaP256 => {
                    let digest = Sha256::digest(message);
                    session
                        .sign(&Mechanism::Ecdsa, handle, &digest)
                        .map_err(device)
                }
                Algorithm::HmacSha256 => session
                    .sign(&Mechanism::Sha256Hmac, handle, message)
                    .map_err(device),
                other => Err(ProviderError::Unsupported(format!(
                    "signing with {other:?}"
                ))),
            }
        })
    }

    fn ecdh(&self, label: &str, peer_point: &[u8]) -> ProviderResult<Zeroizing<Vec<u8>>> {
        if peer_point.len() != 65 || peer_point[0] != 0x04 {
            return Err(ProviderError::Unsupported(
                "a peer key that is not an uncompressed P-256 point".to_string(),
            ));
        }
        self.with_session(|session| {
            let handle = self.find(session, ObjectClass::PRIVATE_KEY, label)?;
            let params = Ecdh1DeriveParams::new(EcKdf::null(), peer_point);
            let template = [
                Attribute::Token(false),
                Attribute::Class(ObjectClass::SECRET_KEY),
                Attribute::KeyType(KeyType::GENERIC_SECRET),
                Attribute::ValueLen(32.into()),
                Attribute::Sensitive(false),
                Attribute::Extractable(true),
            ];
            let derived = session
                .derive_key(&Mechanism::Ecdh1Derive(params), handle, &template)
                .map_err(device)?;
            let value = session
                .get_attributes(derived, &[AttributeType::Value])
                .map_err(device);
            let _ = session.destroy_object(derived);
            match value?.into_iter().next() {
                Some(Attribute::Value(bytes)) => Ok(Zeroizing::new(bytes)),
                _ => Err(ProviderError::Device(
                    "the shared secret was not returned".to_string(),
                )),
            }
        })
    }

    fn encrypt(&self, label: &str, plaintext: &[u8], aad: &[u8]) -> ProviderResult<Vec<u8>> {
        self.with_session(|session| {
            let handle = self.find(session, ObjectClass::SECRET_KEY, label)?;
            let mut iv = session
                .generate_random_vec(GCM_IV_BYTES as u32)
                .map_err(device)?;
            let iv_out = iv.clone();
            let params = GcmParams::new(&mut iv, aad, GCM_TAG_BITS.into()).map_err(device)?;
            let sealed = session
                .encrypt(&Mechanism::AesGcm(params), handle, plaintext)
                .map_err(device)?;
            let mut out = iv_out;
            out.extend(sealed);
            Ok(out)
        })
    }

    fn decrypt(
        &self,
        label: &str,
        sealed: &[u8],
        aad: &[u8],
    ) -> ProviderResult<Zeroizing<Vec<u8>>> {
        if sealed.len() < GCM_IV_BYTES + 16 {
            return Err(ProviderError::Device(
                "the sealed value is too short".to_string(),
            ));
        }
        self.with_session(|session| {
            let handle = self.find(session, ObjectClass::SECRET_KEY, label)?;
            let mut iv = sealed[..GCM_IV_BYTES].to_vec();
            let params = GcmParams::new(&mut iv, aad, GCM_TAG_BITS.into()).map_err(device)?;
            session
                .decrypt(&Mechanism::AesGcm(params), handle, &sealed[GCM_IV_BYTES..])
                .map(Zeroizing::new)
                .map_err(device)
        })
    }

    fn random(&self, length: usize) -> ProviderResult<Vec<u8>> {
        let length = u32::try_from(length).map_err(device)?;
        self.with_session(|session| session.generate_random_vec(length).map_err(device))
    }

    fn destroy(&self, label: &str) -> ProviderResult<()> {
        self.with_session(|session| {
            let handles = session
                .find_objects(&[Attribute::Label(label.as_bytes().to_vec())])
                .map_err(device)?;
            if handles.is_empty() {
                return Err(ProviderError::NoSuchKey(label.to_string()));
            }
            for handle in handles {
                session.destroy_object(handle).map_err(device)?;
            }
            Ok(())
        })
    }

    fn attributes(&self, label: &str) -> ProviderResult<KeyAttributes> {
        self.with_session(|session| {
            let handle = session
                .find_objects(&[Attribute::Label(label.as_bytes().to_vec())])
                .map_err(device)?
                .into_iter()
                .find(|handle| {
                    matches!(
                        session.get_attributes(*handle, &[AttributeType::Class]).ok().and_then(|a| a.into_iter().next()),
                        Some(Attribute::Class(class)) if class == ObjectClass::PRIVATE_KEY || class == ObjectClass::SECRET_KEY
                    )
                })
                .ok_or_else(|| ProviderError::NoSuchKey(label.to_string()))?;
            let attributes = session
                .get_attributes(
                    handle,
                    &[
                        AttributeType::Sensitive,
                        AttributeType::Extractable,
                        AttributeType::AlwaysSensitive,
                        AttributeType::NeverExtractable,
                        AttributeType::Local,
                    ],
                )
                .map_err(device)?;
            let mut out = KeyAttributes {
                sensitive: false,
                extractable: true,
                always_sensitive: false,
                never_extractable: false,
                local: false,
            };
            for attribute in attributes {
                match attribute {
                    Attribute::Sensitive(v) => out.sensitive = v,
                    Attribute::Extractable(v) => out.extractable = v,
                    Attribute::AlwaysSensitive(v) => out.always_sensitive = v,
                    Attribute::NeverExtractable(v) => out.never_extractable = v,
                    Attribute::Local(v) => out.local = v,
                    _ => {}
                }
            }
            Ok(out)
        })
    }
}

fn read_point(
    session: &Session,
    handle: ObjectHandle,
    algorithm: Algorithm,
) -> ProviderResult<Vec<u8>> {
    let raw_length = if algorithm == Algorithm::Ed25519 {
        32
    } else {
        65
    };
    match session
        .get_attributes(handle, &[AttributeType::EcPoint])
        .map_err(device)?
        .into_iter()
        .next()
    {
        Some(Attribute::EcPoint(value)) => unwrap_ec_point(&value, raw_length)
            .ok_or_else(|| ProviderError::Device("unexpected public key encoding".to_string())),
        _ => Err(ProviderError::Device(
            "the token returned no public key".to_string(),
        )),
    }
}
