// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! WASM-compatible local board implementation with stub methods
//!
//! This is a placeholder implementation that allows the Trustee to compile
//! in WASM builds. In the future, this will use IndexedDB for storage.

use std::collections::HashMap;
use std::path::PathBuf;
use strand::context::Ctx;
use strand::hash::Hash;
use b4::messages::artifact::*;
use b4::messages::message::{Message, VerifiedMessage};
use b4::messages::statement::{Statement, StatementType};
use b4::messages::newtypes::*;
use b4::HttpB3Message;
use crate::util::ProtocolError;

/// A WASM-compatible local board that mirrors the structure of LocalBoard
/// but uses browser storage (IndexedDB) instead of SQLite.
pub struct WasmLocalBoard<C: Ctx> {
    pub(crate) configuration: Option<Configuration<C>>,
    cfg_hash: Option<Hash>,
    pub(crate) statements: HashMap<String, (Hash, Statement)>,
    pub(crate) store: Option<PathBuf>,
    artifacts_memory: HashMap<String, (Hash, Vec<u8>)>,
}

impl<C: Ctx> WasmLocalBoard<C> {
    pub(crate) fn new(_store: Option<PathBuf>, _blob_store: Option<PathBuf>) -> Self {
        WasmLocalBoard {
            configuration: None,
            cfg_hash: None,
            statements: HashMap::new(),
            store: None,
            artifacts_memory: HashMap::new(),
        }
    }

    // Stub implementations - these will be implemented properly later
    
    pub(crate) fn store_and_return_messages(
        &mut self,
        _messages: &Vec<HttpB3Message>,
        _last_message_id: i64,
        _ignore_existing: bool,
    ) -> Result<Vec<(Message, i64)>, ProtocolError> {
        Ok(vec![])
    }

    pub(crate) fn update_store(
        &self,
        _messages: &Vec<HttpB3Message>,
        _ignore_existing: bool,
    ) -> Result<(), ProtocolError> {
        Ok(())
    }

    pub(crate) fn get_last_external_id(&self) -> Option<i64> {
        Some(-1)
    }

    pub(crate) fn get_configuration_raw(&self) -> Option<Configuration<C>> {
        self.configuration.clone()
    }

    pub(crate) fn get_cfg_hash(&self) -> Option<Hash> {
        self.cfg_hash.clone()
    }

    pub(crate) fn max_messages(&self) -> usize {
        0
    }

    pub(crate) fn add(&mut self, _message: VerifiedMessage, _id: i64) -> Result<(), ProtocolError> {
        Ok(())
    }

    pub(crate) fn get_configuration(&self, _hash: &ConfigurationHash) -> Option<&Configuration<C>> {
        self.configuration.as_ref()
    }

    pub(crate) fn get_channel(
        &self,
        _hash: &ChannelHash,
        _signer_position: TrusteePosition,
    ) -> Result<Channel<C>, ProtocolError> {
        Err(ProtocolError::WasmNotImplemented)
    }

    pub(crate) fn get_shares(
        &self,
        _hash: &SharesHash,
        _signer_position: TrusteePosition,
    ) -> Result<Shares<C>, ProtocolError> {
        Err(ProtocolError::WasmNotImplemented)
    }

    pub(crate) fn get_dkg_public_key(
        &self,
        _hash: &PublicKeyHash,
        _signer_position: TrusteePosition,
    ) -> Result<DkgPublicKey<C>, ProtocolError> {
        Err(ProtocolError::WasmNotImplemented)
    }

    pub(crate) fn get_ballots(
        &self,
        _hash: &CiphertextsHash,
        _batch: BatchNumber,
        _signer_position: TrusteePosition,
    ) -> Result<Ballots<C>, ProtocolError> {
        Err(ProtocolError::WasmNotImplemented)
    }

    pub(crate) fn get_mix(
        &self,
        _hash: &CiphertextsHash,
        _batch: BatchNumber,
        _signer_position: TrusteePosition,
    ) -> Result<Mix<C>, ProtocolError> {
        Err(ProtocolError::WasmNotImplemented)
    }

    pub(crate) fn get_decryption_factors(
        &self,
        _hash: &DecryptionFactorsHash,
        _batch: BatchNumber,
        _signer_position: TrusteePosition,
    ) -> Result<DecryptionFactors<C>, ProtocolError> {
        Err(ProtocolError::WasmNotImplemented)
    }

    pub(crate) fn get_plaintexts(
        &self,
        _hash: &PlaintextsHash,
        _batch: BatchNumber,
        _signer_position: TrusteePosition,
    ) -> Result<Plaintexts<C>, ProtocolError> {
        Err(ProtocolError::WasmNotImplemented)
    }

    pub(crate) fn get_dkg_public_key_nohash(&self, _signer_position: TrusteePosition) -> Option<DkgPublicKey<C>> {
        None
    }

    pub(crate) fn get_plaintexts_nohash(
        &self,
        _batch: BatchNumber,
        _signer_position: TrusteePosition,
    ) -> Option<Plaintexts<C>> {
        None
    }

    pub(crate) fn get_statement_entries(&self) -> Vec<StatementEntry> {
        vec![]
    }
}

// Placeholder types to match LocalBoard interface
pub struct StatementEntry {
    pub key: StatementEntryIdentifier,
    pub value: (Hash, Statement),
}

pub struct StatementEntryIdentifier {
    pub kind: StatementType,
    pub signer_position: TrusteePosition,
}

