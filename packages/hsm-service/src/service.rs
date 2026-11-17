// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The service's operations, with authorization and audit, independent of
//! the transport. Every operation, allowed or refused, leaves one audit
//! record; an operation whose record can't be written fails and returns
//! nothing.

use std::sync::{Arc, Mutex};

use chrono::Utc;
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

use crate::audit::{AuditLog, Event};
use crate::officials::{StepUpError, StepUpVerifier};
use crate::policy::{Gate, KeyRule, Operation, Policy};
use crate::provider::{Algorithm, KeyAttributes, Provider, ProviderError, ProviderInfo};
use crate::registry::{label, now, KeyRecord, KeyVersion, Registry, VersionState};

/// The longest random value one call returns.
pub const MAX_RANDOM_BYTES: usize = 1024;
/// The most messages one batch signs.
pub const MAX_BATCH: usize = 1024;

#[derive(Debug, thiserror::Error)]
pub enum ServiceError {
    #[error("forbidden: {0}")]
    Forbidden(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error("conflict: {0}")]
    Conflict(String),
    #[error(transparent)]
    StepUp(#[from] StepUpError),
    #[error("device: {0}")]
    Device(String),
    #[error("audit: {0}")]
    Audit(String),
}

impl From<ProviderError> for ServiceError {
    fn from(error: ProviderError) -> Self {
        match error {
            ProviderError::NoSuchKey(k) => ServiceError::NotFound(k),
            ProviderError::Exists(k) => ServiceError::Conflict(k),
            ProviderError::Unsupported(what) => ServiceError::BadRequest(what),
            ProviderError::Device(what) => ServiceError::Device(what),
        }
    }
}

pub type ServiceResult<T> = Result<T, ServiceError>;

/// What binds a gated signature to a signing request.
#[derive(Clone, Debug, Default)]
pub struct Binding {
    pub request_id: Option<String>,
    /// Hex SHA-256 the caller says the message has; checked here.
    pub payload_sha256: Option<String>,
    /// The official's own step-up token, for `official-approval` keys.
    pub official_token: Option<String>,
}

#[derive(Clone, Debug)]
pub struct Signed {
    pub key_version: u32,
    pub signature: Vec<u8>,
}

pub struct Service {
    policy: Policy,
    provider: Arc<dyn Provider>,
    registry: Mutex<Registry>,
    audit: AuditLog,
    officials: Option<StepUpVerifier>,
}

fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

impl Service {
    pub fn new(
        policy: Policy,
        provider: Arc<dyn Provider>,
        registry: Registry,
        audit: AuditLog,
        officials: Option<StepUpVerifier>,
    ) -> Self {
        Service {
            policy,
            provider,
            registry: Mutex::new(registry),
            audit,
            officials,
        }
    }

    pub fn policy(&self) -> &Policy {
        &self.policy
    }

    pub fn info(&self) -> ProviderInfo {
        self.provider.info()
    }

    /// Records a connection whose certificate the CA accepted but the policy
    /// doesn't name.
    pub fn record_unknown_caller(&self, spki_sha256: &str) -> ServiceResult<()> {
        self.record(Event {
            caller: format!("unknown:{spki_sha256}"),
            operation: "connect".to_string(),
            outcome: "refused: the policy names no caller with this key".to_string(),
            ..Default::default()
        })
    }

    fn record(&self, event: Event) -> ServiceResult<()> {
        self.audit
            .append(event)
            .map(|_| ())
            .map_err(|e| ServiceError::Audit(e.to_string()))
    }

    fn refuse<T>(&self, mut event: Event, error: ServiceError) -> ServiceResult<T> {
        event.outcome = format!("refused: {error}");
        self.record(event)?;
        Err(error)
    }

    fn registry(&self) -> ServiceResult<std::sync::MutexGuard<'_, Registry>> {
        self.registry
            .lock()
            .map_err(|_| ServiceError::Device("registry lock".to_string()))
    }

    fn admin_check(&self, caller: &str, event: &Event) -> ServiceResult<()> {
        if self.policy.is_administrator(caller) {
            Ok(())
        } else {
            self.refuse(
                event.clone(),
                ServiceError::Forbidden("not a key administrator".to_string()),
            )
        }
    }

    /// Generates a key's first version.
    pub fn create_key(
        &self,
        caller: &str,
        key_id: &str,
        algorithm: Algorithm,
    ) -> ServiceResult<KeyRecord> {
        let event = Event {
            caller: caller.to_string(),
            operation: "create-key".to_string(),
            key_id: Some(key_id.to_string()),
            key_version: Some(1),
            ..Default::default()
        };
        self.admin_check(caller, &event)?;
        match self.policy.rule(key_id) {
            Some(rule) if rule.algorithm == algorithm => {}
            Some(_) => {
                return self.refuse(
                    event,
                    ServiceError::BadRequest("the policy names another algorithm".to_string()),
                )
            }
            None => {
                return self.refuse(
                    event,
                    ServiceError::Forbidden("no policy for this key".to_string()),
                )
            }
        }
        if key_id.contains('#') {
            return self.refuse(
                event,
                ServiceError::BadRequest("key ids can't contain '#'".to_string()),
            );
        }
        let mut registry = self.registry()?;
        if registry.get(key_id).is_some() {
            return self.refuse(event, ServiceError::Conflict(key_id.to_string()));
        }
        let version = self.new_version(key_id, algorithm, 1)?;
        let record = KeyRecord {
            key_id: key_id.to_string(),
            algorithm,
            versions: vec![version],
        };
        registry
            .put(record.clone())
            .map_err(|e| ServiceError::Device(e.to_string()))?;
        self.record(Event {
            outcome: "ok".to_string(),
            ..event
        })?;
        Ok(record)
    }

    fn new_version(
        &self,
        key_id: &str,
        algorithm: Algorithm,
        version: u32,
    ) -> ServiceResult<KeyVersion> {
        let label = label(key_id, version);
        let public = self.provider.generate(&label, algorithm)?;
        Ok(KeyVersion {
            version,
            label,
            state: VersionState::Active,
            created_at: now(),
            retired_at: None,
            destroyed_at: None,
            public_key: hex::encode(public),
        })
    }

    /// Adds a version and retires the active one.
    pub fn rotate(&self, caller: &str, key_id: &str) -> ServiceResult<KeyRecord> {
        let mut event = Event {
            caller: caller.to_string(),
            operation: "rotate".to_string(),
            key_id: Some(key_id.to_string()),
            ..Default::default()
        };
        self.admin_check(caller, &event)?;
        let mut registry = self.registry()?;
        let Some(mut record) = registry.get(key_id).cloned() else {
            return self.refuse(event, ServiceError::NotFound(key_id.to_string()));
        };
        let next = record.next_version();
        event.key_version = Some(next);
        let version = self.new_version(key_id, record.algorithm, next)?;
        let at = now();
        for existing in record.versions.iter_mut() {
            if existing.state == VersionState::Active {
                existing.state = VersionState::Retired;
                existing.retired_at = Some(at.clone());
            }
        }
        record.versions.push(version);
        registry
            .put(record.clone())
            .map_err(|e| ServiceError::Device(e.to_string()))?;
        self.record(Event {
            outcome: "ok".to_string(),
            ..event
        })?;
        Ok(record)
    }

    pub fn retire(&self, caller: &str, key_id: &str, version: u32) -> ServiceResult<KeyRecord> {
        self.change_state(caller, key_id, version, VersionState::Retired)
    }

    /// Removes a retired version's device object. The record stays, so its
    /// signatures still verify against the listed public key.
    pub fn destroy(&self, caller: &str, key_id: &str, version: u32) -> ServiceResult<KeyRecord> {
        self.change_state(caller, key_id, version, VersionState::Destroyed)
    }

    fn change_state(
        &self,
        caller: &str,
        key_id: &str,
        version: u32,
        to: VersionState,
    ) -> ServiceResult<KeyRecord> {
        let event = Event {
            caller: caller.to_string(),
            operation: match to {
                VersionState::Retired => "retire",
                VersionState::Destroyed => "destroy",
                VersionState::Active => "activate",
            }
            .to_string(),
            key_id: Some(key_id.to_string()),
            key_version: Some(version),
            ..Default::default()
        };
        self.admin_check(caller, &event)?;
        let mut registry = self.registry()?;
        let Some(mut record) = registry.get(key_id).cloned() else {
            return self.refuse(event, ServiceError::NotFound(key_id.to_string()));
        };
        let Some(entry) = record.versions.iter_mut().find(|v| v.version == version) else {
            return self.refuse(
                event,
                ServiceError::NotFound(format!("{key_id} v{version}")),
            );
        };
        let allowed = matches!(
            (entry.state, to),
            (VersionState::Active, VersionState::Retired)
                | (VersionState::Retired, VersionState::Destroyed)
        );
        if !allowed {
            return self.refuse(
                event,
                ServiceError::Conflict(format!(
                    "v{version} is {:?}; only active→retired→destroyed",
                    entry.state
                )),
            );
        }
        if to == VersionState::Destroyed {
            self.provider.destroy(&entry.label)?;
            entry.destroyed_at = Some(now());
        } else {
            entry.retired_at = Some(now());
        }
        entry.state = to;
        registry
            .put(record.clone())
            .map_err(|e| ServiceError::Device(e.to_string()))?;
        self.record(Event {
            outcome: "ok".to_string(),
            ..event
        })?;
        Ok(record)
    }

    /// Public versions of a key, for any registered caller.
    pub fn describe(&self, caller: &str, key_id: &str) -> ServiceResult<KeyRecord> {
        let event = Event {
            caller: caller.to_string(),
            operation: "describe".to_string(),
            key_id: Some(key_id.to_string()),
            ..Default::default()
        };
        let registry = self.registry()?;
        match registry.get(key_id) {
            Some(record) => Ok(record.clone()),
            None => self.refuse(event, ServiceError::NotFound(key_id.to_string())),
        }
    }

    pub fn list(&self) -> ServiceResult<Vec<KeyRecord>> {
        Ok(self.registry()?.all().cloned().collect())
    }

    pub fn attributes(
        &self,
        caller: &str,
        key_id: &str,
        version: u32,
    ) -> ServiceResult<KeyAttributes> {
        let event = Event {
            caller: caller.to_string(),
            operation: "attributes".to_string(),
            key_id: Some(key_id.to_string()),
            key_version: Some(version),
            ..Default::default()
        };
        self.admin_check(caller, &event)?;
        let label = {
            let registry = self.registry()?;
            registry
                .get(key_id)
                .and_then(|r| r.version(version))
                .map(|v| v.label.clone())
                .ok_or_else(|| ServiceError::NotFound(format!("{key_id} v{version}")))?
        };
        Ok(self.provider.attributes(&label)?)
    }

    /// Checks the caller, purpose and operation; returns the rule and the
    /// active version's label and number.
    fn authorize(
        &self,
        caller: &str,
        key_id: &str,
        purpose: &str,
        operation: Operation,
        event: &Event,
    ) -> ServiceResult<(KeyRule, String, u32, Algorithm)> {
        let Some(rule) = self.policy.rule(key_id).cloned() else {
            return self.refuse(
                event.clone(),
                ServiceError::Forbidden("no policy for this key".to_string()),
            );
        };
        if !rule.users.contains(caller) {
            return self.refuse(
                event.clone(),
                ServiceError::Forbidden("not a user of this key".to_string()),
            );
        }
        if !rule.purposes.contains(purpose) {
            return self.refuse(
                event.clone(),
                ServiceError::Forbidden(format!("purpose '{purpose}' not allowed")),
            );
        }
        if !rule.operations.contains(&operation) {
            return self.refuse(
                event.clone(),
                ServiceError::Forbidden(format!("{operation:?} not allowed")),
            );
        }
        let registry = self.registry()?;
        let Some(record) = registry.get(key_id) else {
            return self.refuse(event.clone(), ServiceError::NotFound(key_id.to_string()));
        };
        let Some(active) = record.active() else {
            return self.refuse(
                event.clone(),
                ServiceError::Conflict("the key has no active version".to_string()),
            );
        };
        Ok((rule, active.label.clone(), active.version, record.algorithm))
    }

    async fn check_gate(
        &self,
        rule: &KeyRule,
        caller: &str,
        key_id: &str,
        message: &[u8],
        binding: &Binding,
        event: &mut Event,
    ) -> ServiceResult<()> {
        let digest_matches = || {
            binding.payload_sha256.as_deref().map(str::to_lowercase) == Some(sha256_hex(message))
        };
        match &rule.gate {
            Gate::None => Ok(()),
            Gate::CompletedExecution { executors } => {
                if !executors.contains(caller) {
                    return self.refuse(
                        event.clone(),
                        ServiceError::Forbidden("not an executor".to_string()),
                    );
                }
                if binding.request_id.is_none() || !digest_matches() {
                    return self.refuse(
                        event.clone(),
                        ServiceError::BadRequest(
                            "an execution must name its request and the message's digest"
                                .to_string(),
                        ),
                    );
                }
                Ok(())
            }
            Gate::OfficialApproval { key_id_prefix } => {
                if binding.request_id.is_none() || !digest_matches() {
                    return self.refuse(
                        event.clone(),
                        ServiceError::BadRequest(
                            "an approval must name its request and the message's digest"
                                .to_string(),
                        ),
                    );
                }
                let (Some(verifier), Some(token)) = (&self.officials, &binding.official_token)
                else {
                    return self
                        .refuse(event.clone(), ServiceError::StepUp(StepUpError::NotStepUp));
                };
                let official = match verifier.verify(token, Utc::now().timestamp()).await {
                    Ok(official) => official,
                    Err(error) => return self.refuse(event.clone(), error.into()),
                };
                event.official = Some(official.subject.clone());
                if key_id.strip_prefix(key_id_prefix.as_str()) != Some(official.subject.as_str()) {
                    return self.refuse(
                        event.clone(),
                        ServiceError::Forbidden(
                            "an official may only use their own key".to_string(),
                        ),
                    );
                }
                Ok(())
            }
        }
    }

    pub async fn sign(
        &self,
        caller: &str,
        key_id: &str,
        purpose: &str,
        message: &[u8],
        binding: &Binding,
    ) -> ServiceResult<Signed> {
        let mut event = Event {
            caller: caller.to_string(),
            operation: "sign".to_string(),
            key_id: Some(key_id.to_string()),
            purpose: Some(purpose.to_string()),
            message_sha256: vec![sha256_hex(message)],
            request_id: binding.request_id.clone(),
            ..Default::default()
        };
        let (rule, label, version, algorithm) =
            self.authorize(caller, key_id, purpose, Operation::Sign, &event)?;
        event.key_version = Some(version);
        self.check_gate(&rule, caller, key_id, message, binding, &mut event)
            .await?;
        let provider = self.provider.clone();
        let owned = message.to_vec();
        let signature =
            tokio::task::spawn_blocking(move || provider.sign(&label, algorithm, &owned))
                .await
                .map_err(|e| ServiceError::Device(e.to_string()))?;
        match signature {
            Ok(signature) => {
                self.record(Event {
                    outcome: "ok".to_string(),
                    ..event
                })?;
                Ok(Signed {
                    key_version: version,
                    signature,
                })
            }
            Err(error) => self.refuse(event, error.into()),
        }
    }

    /// Signs each message on its own, in order: one signature per message,
    /// as if each were signed alone. Only for ungated system keys.
    pub async fn sign_batch(
        &self,
        caller: &str,
        key_id: &str,
        purpose: &str,
        messages: &[Vec<u8>],
    ) -> ServiceResult<(u32, Vec<Vec<u8>>)> {
        let event = Event {
            caller: caller.to_string(),
            operation: "sign-batch".to_string(),
            key_id: Some(key_id.to_string()),
            purpose: Some(purpose.to_string()),
            message_sha256: messages.iter().map(|m| sha256_hex(m)).collect(),
            ..Default::default()
        };
        if messages.is_empty() || messages.len() > MAX_BATCH {
            return self.refuse(
                event,
                ServiceError::BadRequest(format!("a batch holds 1 to {MAX_BATCH} messages")),
            );
        }
        let (rule, label, version, algorithm) =
            self.authorize(caller, key_id, purpose, Operation::Sign, &event)?;
        if rule.gate != Gate::None {
            return self.refuse(
                event,
                ServiceError::Forbidden("gated keys sign one message at a time".to_string()),
            );
        }
        let provider = self.provider.clone();
        let owned = messages.to_vec();
        let signatures = tokio::task::spawn_blocking(move || {
            owned
                .iter()
                .map(|m| provider.sign(&label, algorithm, m))
                .collect::<Result<Vec<_>, _>>()
        })
        .await
        .map_err(|e| ServiceError::Device(e.to_string()))?;
        let event = Event {
            key_version: Some(version),
            ..event
        };
        match signatures {
            Ok(signatures) => {
                self.record(Event {
                    outcome: "ok".to_string(),
                    ..event
                })?;
                Ok((version, signatures))
            }
            Err(error) => self.refuse(event, error.into()),
        }
    }

    pub async fn ecdh(
        &self,
        caller: &str,
        key_id: &str,
        purpose: &str,
        peer_point: &[u8],
        binding: &Binding,
    ) -> ServiceResult<(u32, Zeroizing<Vec<u8>>)> {
        let mut event = Event {
            caller: caller.to_string(),
            operation: "ecdh".to_string(),
            key_id: Some(key_id.to_string()),
            purpose: Some(purpose.to_string()),
            message_sha256: vec![sha256_hex(peer_point)],
            request_id: binding.request_id.clone(),
            ..Default::default()
        };
        let (rule, label, version, _) =
            self.authorize(caller, key_id, purpose, Operation::Ecdh, &event)?;
        event.key_version = Some(version);
        self.check_gate(&rule, caller, key_id, peer_point, binding, &mut event)
            .await?;
        let provider = self.provider.clone();
        let peer = peer_point.to_vec();
        let shared = tokio::task::spawn_blocking(move || provider.ecdh(&label, &peer))
            .await
            .map_err(|e| ServiceError::Device(e.to_string()))?;
        match shared {
            Ok(shared) => {
                self.record(Event {
                    outcome: "ok".to_string(),
                    ..event
                })?;
                Ok((version, shared))
            }
            Err(error) => self.refuse(event, error.into()),
        }
    }

    /// Seals with the active version; the version is returned so it can be
    /// stored next to the sealed value.
    pub async fn encrypt(
        &self,
        caller: &str,
        key_id: &str,
        purpose: &str,
        plaintext: &[u8],
        aad: &[u8],
    ) -> ServiceResult<(u32, Vec<u8>)> {
        let mut event = Event {
            caller: caller.to_string(),
            operation: "encrypt".to_string(),
            key_id: Some(key_id.to_string()),
            purpose: Some(purpose.to_string()),
            message_sha256: vec![sha256_hex(aad)],
            ..Default::default()
        };
        let (rule, label, version, _) =
            self.authorize(caller, key_id, purpose, Operation::Encrypt, &event)?;
        event.key_version = Some(version);
        if rule.gate != Gate::None {
            return self.refuse(
                event,
                ServiceError::Forbidden("sealing keys are ungated".to_string()),
            );
        }
        let provider = self.provider.clone();
        let (plain, extra) = (Zeroizing::new(plaintext.to_vec()), aad.to_vec());
        let sealed = tokio::task::spawn_blocking(move || provider.encrypt(&label, &plain, &extra))
            .await
            .map_err(|e| ServiceError::Device(e.to_string()))?;
        match sealed {
            Ok(sealed) => {
                self.record(Event {
                    outcome: "ok".to_string(),
                    ..event
                })?;
                Ok((version, sealed))
            }
            Err(error) => self.refuse(event, error.into()),
        }
    }

    /// Opens a value sealed by any version that hasn't been destroyed.
    pub async fn decrypt(
        &self,
        caller: &str,
        key_id: &str,
        purpose: &str,
        version: u32,
        sealed: &[u8],
        aad: &[u8],
    ) -> ServiceResult<Zeroizing<Vec<u8>>> {
        let event = Event {
            caller: caller.to_string(),
            operation: "decrypt".to_string(),
            key_id: Some(key_id.to_string()),
            key_version: Some(version),
            purpose: Some(purpose.to_string()),
            message_sha256: vec![sha256_hex(aad)],
            ..Default::default()
        };
        let (rule, _, _, _) =
            self.authorize(caller, key_id, purpose, Operation::Decrypt, &event)?;
        if rule.gate != Gate::None {
            return self.refuse(
                event,
                ServiceError::Forbidden("sealing keys are ungated".to_string()),
            );
        }
        let label = {
            let registry = self.registry()?;
            match registry.get(key_id).and_then(|r| r.version(version)) {
                Some(v) if v.state != VersionState::Destroyed => v.label.clone(),
                _ => {
                    return self.refuse(
                        event,
                        ServiceError::NotFound(format!("{key_id} v{version}")),
                    )
                }
            }
        };
        let provider = self.provider.clone();
        let (blob, extra) = (sealed.to_vec(), aad.to_vec());
        let opened = tokio::task::spawn_blocking(move || provider.decrypt(&label, &blob, &extra))
            .await
            .map_err(|e| ServiceError::Device(e.to_string()))?;
        match opened {
            Ok(opened) => {
                self.record(Event {
                    outcome: "ok".to_string(),
                    ..event
                })?;
                Ok(opened)
            }
            Err(error) => self.refuse(event, error.into()),
        }
    }

    /// Randomness from the device, for any registered caller.
    pub async fn random(&self, caller: &str, length: usize) -> ServiceResult<Vec<u8>> {
        let event = Event {
            caller: caller.to_string(),
            operation: "random".to_string(),
            ..Default::default()
        };
        if length == 0 || length > MAX_RANDOM_BYTES {
            return self.refuse(
                event,
                ServiceError::BadRequest(format!("1 to {MAX_RANDOM_BYTES} bytes")),
            );
        }
        let provider = self.provider.clone();
        let bytes = tokio::task::spawn_blocking(move || provider.random(length))
            .await
            .map_err(|e| ServiceError::Device(e.to_string()))??;
        self.record(Event {
            outcome: "ok".to_string(),
            ..event
        })?;
        Ok(bytes)
    }
}
