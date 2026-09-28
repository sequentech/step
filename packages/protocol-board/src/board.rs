// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Communication with the board service.
//!
//! The board service is untrusted: it stores bytes and hands them back, and
//! nothing it says is believed until braid has verified it against the
//! `Configuration` the platform posted. This module is where that boundary is
//! drawn, and it is the only part of this crate that does any I/O.
//!
//! # Outage or verdict
//!
//! Reading a board is done in two steps:
//!
//! First, the messages are fetched over HTTP; anything that goes wrong
//! there is an outage, returned as an error, and the ceremony simply tries
//! again on the next beat.
//!
//! Then, those same messages are handed to braid's own board client over an
//! in-memory board, where no network exists, so anything that goes wrong
//! *there* is about the content and final, and comes back as a
//! [`DkgOutcome::Unusable`](crate::view::DkgStatus::Unusable) reading.

use std::future::Future;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use sequent_core::types::ceremonies::KeysCeremonyFailureReason;
use tokio::runtime::Handle;
use tokio::task::spawn_blocking;
use tracing::{instrument, warn};
use wbraid::board::persistence::NoOpPersistence;
use wbraid::board::transport::{MemoryBoard, MemoryTransport, Transport};
use wbraid::board::BoardClient;
use wbraid::messages::newtypes::ConfigurationHash;
use wbraid::native::http_transport::HttpTransport;

use crate::configuration::SignedConfiguration;
use crate::encoding::HashHex;
use crate::ids::BoardName;
use crate::view::DkgView;
use crate::Ctx;

/// One board on the board service.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoardHandle {
    base_url: String,
    board: BoardName,
}

impl BoardHandle {
    pub fn new(base_url: &str, board: BoardName) -> BoardHandle {
        BoardHandle {
            base_url: base_url.to_string(),
            board,
        }
    }

    /// Create the board and post the ceremony's stored `Configuration` message.
    ///
    /// Safe to repeat: creating a board twice harms nothing, and posting the
    /// same bytes again leaves the board serving what it served. Whether the
    /// board serves this ceremony's `Configuration` is checked by every reading,
    /// not here.
    /// An error means an outage.
    #[instrument(skip(self, configuration), fields(board = %self.board), err)]
    pub async fn publish_configuration(
        &self,
        configuration: &SignedConfiguration,
    ) -> Result<()> {
        let base_url = self.base_url.clone();
        let board = self.board.clone();
        let message = configuration.message().clone();

        run_board_io(move || async move {
            // Board creation is untrusted namespacing: the board service
            // refuses a board that already exists, which could happen with a retry.
            if let Err(err) =
                HttpTransport::create_board(&base_url, board.as_str()).await
            {
                warn!("could not create board {board}: {err:#}");
            }
            let transport = HttpTransport::new(&base_url, board.as_str());
            Transport::<Ctx>::publish(&transport, &message).await
        })
        .await
    }

    /// Read the board and say where the key generation stands. An error is an
    /// outage: nothing is concluded from it.
    #[instrument(skip(self, expected_cfg), fields(board = %self.board), err)]
    pub async fn fetch_dkg_status(
        &self,
        expected_cfg: &HashHex,
    ) -> Result<DkgView> {
        let base_url = self.base_url.clone();
        let board = self.board.clone();
        let expected_cfg = expected_cfg.clone();

        run_board_io(move || async move {
            let transport = HttpTransport::new(&base_url, board.as_str());
            let configuration =
                Transport::<Ctx>::fetch_configuration(&transport).await?;
            let messages = Transport::<Ctx>::fetch(&transport).await?;

            let fetched = MemoryBoard::<Ctx>::new();
            fetched.push(configuration);
            for message in messages {
                fetched.push(message);
            }
            Ok(read_dkg(fetched, &expected_cfg).await)
        })
        .await
    }
}

/// Read a board that has already been fetched. Nothing here touches the
/// network, so every failure is about what the board carries.
pub(crate) async fn read_dkg(
    board: Arc<MemoryBoard<Ctx>>,
    expected_cfg: &HashHex,
) -> DkgView {
    async move {
        let mut client =
            BoardClient::connect(MemoryTransport::new(board), NoOpPersistence)
                .await?;
        let served =
            ConfigurationHash::from_configuration(client.configuration())?;
        let served_hex = HashHex::of(&served.0);
        if served_hex != *expected_cfg {
            return Ok(DkgView::unusable(
                KeysCeremonyFailureReason::BOARD_CONFIGURATION_MISMATCH,
                format!(
                    "the board serves configuration {}, not this ceremony's {}",
                    served_hex.short(),
                    expected_cfg.short()
                ),
            ));
        }
        client.update().await?;
        Ok(DkgView::from_verified(&served, client.view()))
    }
    .await
    .unwrap_or_else(|err: anyhow::Error| {
        DkgView::unusable(
            KeysCeremonyFailureReason::INVALID_BOARD_CONTENT,
            format!("{err:#}"),
        )
    })
}

/// Drive a braid board call to completion from an async task.
///
/// A worker that panics comes back as an outage, so a bug there stops
/// the ceremony from advancing.
///
/// TODO: Remove once wbraid is updated to be Send compatible.
async fn run_board_io<T, F, Fut>(call: F) -> Result<T>
where
    F: FnOnce() -> Fut + Send + 'static,
    Fut: Future<Output = Result<T>>,
    T: Send + 'static,
{
    let handle = Handle::current();
    spawn_blocking(move || handle.block_on(call()))
        .await
        .map_err(|err| anyhow!("the board transport worker failed: {err}"))?
}
