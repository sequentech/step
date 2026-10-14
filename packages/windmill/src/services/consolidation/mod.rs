// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Consolidation of tally results into transmission packages.
//!
//! The modules that build, sign and send the Miru (ACM/EML) transmission
//! packages depend on the external ECIES tool and are compiled only with the
//! `miru` feature; they are not part of the evaluated TOE. Without the
//! feature, the entry points used by the Celery tasks, the HTTP routes and
//! the signing services are stubs that return an error, so that they compile
//! unchanged and no cryptographic code of the integration is built.

pub mod aes_256_cbc_encrypt;
pub mod eml_generator;
pub mod eml_types;
pub mod tally_download;
pub mod xz_compress;
pub mod zip;

#[cfg(feature = "miru")]
pub mod acm_json;
#[cfg(feature = "miru")]
pub mod acm_transaction;
#[cfg(feature = "miru")]
pub mod create_transmission_package_service;
#[cfg(feature = "miru")]
pub mod logs;
#[cfg(feature = "miru")]
pub mod package_manifest;
#[cfg(feature = "miru")]
pub mod rsa;
#[cfg(feature = "miru")]
pub mod send_transmission_package_service;
#[cfg(feature = "miru")]
pub mod signatures;
#[cfg(feature = "miru")]
pub mod signed_transmission_package;
#[cfg(feature = "miru")]
pub mod transmission_package;
#[cfg(feature = "miru")]
pub mod upload_signature_service;

#[cfg(not(feature = "miru"))]
pub(crate) const MIRU_DISABLED: &str =
    "The Miru integration is not part of this build (feature `miru` is disabled)";

#[cfg(not(feature = "miru"))]
pub mod create_transmission_package_service {
    pub async fn create_transmission_package_service(
        _tenant_id: &str,
        _election_id: &str,
        _area_id: &str,
        _tally_session_id: &str,
        _force: bool,
        _requester: Option<crate::services::signing::SigningCaller>,
    ) -> anyhow::Result<()> {
        Err(anyhow::anyhow!(super::MIRU_DISABLED))
    }
}

#[cfg(not(feature = "miru"))]
pub mod send_transmission_package_service {
    use crate::services::signing::actions::transmission::TransmissionRefusal;

    /// A send refused because the package has fewer signatures than it needs.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct TransmissionSignaturesShort {
        pub signatures: usize,
        pub threshold: i64,
    }

    impl std::fmt::Display for TransmissionSignaturesShort {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(
                f,
                "The transmission package has {} of the {} signatures it needs; it can't be sent yet",
                self.signatures, self.threshold
            )
        }
    }

    impl std::error::Error for TransmissionSignaturesShort {}

    /// Why a send can't be queued.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum SendRefusal {
        /// There is no such tally session in the event.
        NoTallySession,
        Refused(TransmissionRefusal),
    }

    pub async fn signed_transmission_refusal(
        _tenant_id: &str,
        _election_id: &str,
        _area_id: &str,
        _tally_session_id: &str,
    ) -> anyhow::Result<Option<SendRefusal>> {
        Err(anyhow::anyhow!(super::MIRU_DISABLED))
    }

    pub async fn send_transmission_package_service(
        _tenant_id: &str,
        _election_id: &str,
        _area_id: &str,
        _tally_session_id: &str,
    ) -> anyhow::Result<()> {
        Err(anyhow::anyhow!(super::MIRU_DISABLED))
    }
}

/// Without the feature, the EML signatures of transmission requests are
/// refused: the Post's transmission configuration is Miru data.
#[cfg(not(feature = "miru"))]
pub mod signed_transmission_package {
    use crate::postgres::signing::SigningRequestRow;
    use crate::services::signing::actions::transmission::{PostSbeis, SbeiDirectory};
    use async_trait::async_trait;
    use deadpool_postgres::Transaction;

    #[derive(Debug, Clone, Copy, Default)]
    pub struct AnnotatedSbeis;

    #[async_trait]
    impl SbeiDirectory for AnnotatedSbeis {
        async fn post_sbeis(
            &self,
            _hasura_transaction: &Transaction<'_>,
            _request: &SigningRequestRow,
        ) -> anyhow::Result<Option<PostSbeis>> {
            Err(anyhow::anyhow!(super::MIRU_DISABLED))
        }
    }
}

#[cfg(not(feature = "miru"))]
pub mod upload_signature_service {
    pub async fn upload_transmission_package_signature_service(
        _tenant_id: &str,
        _election_id: &str,
        _area_id: &str,
        _tally_session_id: &str,
        _username: &str,
        _document_id: &str,
        _password: &str,
    ) -> anyhow::Result<()> {
        Err(anyhow::anyhow!(super::MIRU_DISABLED))
    }
}
