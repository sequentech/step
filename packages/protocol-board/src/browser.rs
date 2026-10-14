// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Browser trustees use the same sessions and custody proofs as native trustees.

use crate::trustee_sessions::{BoardAccess, SessionSet};
use crate::{BoardName, CustodyRequest, TrusteeBoard, TrusteeSecrets};
use anyhow::{ensure, Result};
use cryptography::utils::serialization::{Deserializable, Serializable};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use wbraid::board::persistence::Persistence;
use wbraid::board::transport::StagedRef;
use wbraid::messages::predicate::Predicate;
use wbraid::wasm::{
    persistence::IndexedDbPersistence, transport::WasmHttpTransport,
};

struct BrowserBoards {
    url: String,
    namespace: String,
}

#[async_trait::async_trait(?Send)]
impl BoardAccess for BrowserBoards {
    type Transport = WasmHttpTransport;
    type Persistence = IndexedDbPersistence;
    fn transport(&self, board: &BoardName) -> Self::Transport {
        WasmHttpTransport::new(&self.url, board.as_str())
    }
    async fn store(&self, board: &BoardName) -> Result<Self::Persistence> {
        IndexedDbPersistence::open(&format!("{}:{}", self.namespace, board))
            .await
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoreBackup {
    board: BoardName,
    predicates: Vec<String>,
    own_posts: Vec<(String, String)>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Backup {
    version: u32,
    keys: String,
    board_url: String,
    stores: Vec<StoreBackup>,
}

fn js_error(error: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&error.to_string())
}
fn predicate(encoded: &str) -> Result<Predicate> {
    Ok(Predicate::deser(&hex::decode(encoded)?)?)
}

#[wasm_bindgen]
pub struct BrowserTrustee {
    secrets: TrusteeSecrets,
    access: BrowserBoards,
    sessions: SessionSet<BrowserBoards>,
}

#[wasm_bindgen]
impl BrowserTrustee {
    /// Generate the same owner-only TOML identity the native CLI accepts.
    #[wasm_bindgen(js_name = generateKeys)]
    pub fn generate_keys() -> Result<String, JsValue> {
        TrusteeSecrets::generate().to_toml().map_err(js_error)
    }
    #[wasm_bindgen(constructor)]
    pub fn new(
        name: String,
        keys: &str,
        board_url: String,
    ) -> Result<BrowserTrustee, JsValue> {
        let secrets = TrusteeSecrets::parse(keys).map_err(js_error)?;
        let public = secrets.public_keys().map_err(js_error)?;
        let url = board_url.trim_end_matches('/').to_string();
        let namespace =
            format!("sequent-wbraid:{}:{}", url, public.signing_public_key);
        let access = BrowserBoards {
            url: url.clone(),
            namespace: namespace.clone(),
        };
        let sessions = SessionSet::new(
            name,
            secrets.clone(),
            BrowserBoards { url, namespace },
        );
        Ok(Self {
            secrets,
            access,
            sessions,
        })
    }
    #[wasm_bindgen(js_name = publicKeys)]
    pub fn public_keys(&self) -> Result<String, JsValue> {
        let keys = self.secrets.public_keys().map_err(js_error)?;
        serde_json::to_string(&serde_json::json!({
            "signing_public_key": keys.signing_public_key,
            "share_encryption_public_key": keys.share_encryption_public_key,
        }))
        .map_err(js_error)
    }
    pub fn attest(&self, request: &str) -> Result<String, JsValue> {
        let request: CustodyRequest =
            serde_json::from_str(request).map_err(js_error)?;
        request.attest(&self.secrets).map_err(js_error)
    }
    /// Called sequentially. Halted boards stay halted, including between retries.
    pub async fn advance(&mut self, boards: &str) -> Result<String, JsValue> {
        let boards: Vec<TrusteeBoard> =
            serde_json::from_str(boards).map_err(js_error)?;
        self.sessions.reconcile(&boards).await;
        self.sessions.advance().await;
        serde_json::to_string(&self.sessions.pending_reports())
            .map_err(js_error)
    }
    pub fn settle(&mut self, board: &str) -> Result<(), JsValue> {
        self.sessions
            .settle(&BoardName::parse(board).map_err(js_error)?);
        Ok(())
    }
    /// Includes irreversible commitments and staged-message handles, not just keys.
    pub async fn backup(&self, boards: &str) -> Result<String, JsValue> {
        let boards: Vec<BoardName> =
            serde_json::from_str(boards).map_err(js_error)?;
        let mut stores = Vec::new();
        for board in boards {
            let store = self.access.store(&board).await.map_err(js_error)?;
            stores.push(StoreBackup {
                board,
                predicates: store
                    .load()
                    .await
                    .map_err(js_error)?
                    .iter()
                    .map(|p| hex::encode(p.ser()))
                    .collect(),
                own_posts: store
                    .load_own_posts()
                    .await
                    .map_err(js_error)?
                    .iter()
                    .map(|(p, s)| (hex::encode(p.ser()), s.0.clone()))
                    .collect(),
            });
        }
        serde_json::to_string(&Backup {
            version: 1,
            keys: self.secrets.to_toml().map_err(js_error)?,
            board_url: self.access.url.clone(),
            stores,
        })
        .map_err(js_error)
    }
    /// Parse all records before writing. Existing commitments are never cleared.
    #[wasm_bindgen(js_name = restoreBackup)]
    pub async fn restore_backup(
        &self,
        contents: &str,
    ) -> Result<String, JsValue> {
        self.restore(contents).await.map_err(js_error)
    }
}

impl BrowserTrustee {
    async fn restore(&self, contents: &str) -> Result<String> {
        let backup: Backup = serde_json::from_str(contents)?;
        ensure!(
            backup.version == 1 && backup.board_url == self.access.url,
            "backup version or board service mismatch"
        );
        ensure!(
            TrusteeSecrets::parse(&backup.keys)?.public_keys()?
                == self.secrets.public_keys()?,
            "backup keys mismatch"
        );
        let mut records = Vec::new();
        for entry in &backup.stores {
            let predicates = entry
                .predicates
                .iter()
                .map(|p| predicate(p))
                .collect::<Result<Vec<_>>>()?;
            let own_posts = entry
                .own_posts
                .iter()
                .map(|(p, s)| Ok((predicate(p)?, StagedRef(s.clone()))))
                .collect::<Result<Vec<_>>>()?;
            records.push((entry.board.clone(), predicates, own_posts));
        }
        // Check the entire import against local state before any write.
        for (board, _, posts) in &records {
            let store = self.access.store(board).await?;
            let local = store.load_own_posts().await?;
            for (predicate, staged) in posts {
                ensure!(
                    !local.iter().any(|(p, s)| p == predicate && s != staged),
                    "backup conflicts with a locally staged message"
                );
                ensure!(
                    !posts.iter().any(|(p, s)| p == predicate && s != staged),
                    "backup contains conflicting staged messages"
                );
            }
        }
        for (board, predicates, posts) in records {
            let mut store = self.access.store(&board).await?;
            for p in predicates {
                store.persist(&p).await?;
            }
            for (p, s) in posts {
                store.persist_own_post(&p, &s).await?;
            }
        }
        Ok(serde_json::to_string(
            &backup
                .stores
                .iter()
                .map(|entry| &entry.board)
                .collect::<Vec<_>>(),
        )?)
    }
}
