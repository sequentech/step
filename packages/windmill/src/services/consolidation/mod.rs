// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Consolidation of tally results into transmission packages.
//!
//! The modules that build, sign and send the Miru (ACM/EML) transmission
//! packages depend on the external ECIES tool and are compiled only with the
//! `miru` feature; they are not part of the evaluated TOE. Without the
//! feature, the three entry points used by the Celery tasks are stubs that
//! return an error, so that the task registry and the HTTP routes compile
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
pub mod rsa;
#[cfg(feature = "miru")]
pub mod send_transmission_package_service;
#[cfg(feature = "miru")]
pub mod signatures;
#[cfg(feature = "miru")]
pub mod transmission_package;
#[cfg(feature = "miru")]
pub mod upload_signature_service;

#[cfg(not(feature = "miru"))]
const MIRU_DISABLED: &str =
    "The Miru integration is not part of this build (feature `miru` is disabled)";

#[cfg(not(feature = "miru"))]
pub mod create_transmission_package_service {
    pub async fn create_transmission_package_service(
        _tenant_id: &str,
        _election_id: &str,
        _area_id: &str,
        _tally_session_id: &str,
        _force: bool,
    ) -> anyhow::Result<()> {
        Err(anyhow::anyhow!(super::MIRU_DISABLED))
    }
}

#[cfg(not(feature = "miru"))]
pub mod send_transmission_package_service {
    pub async fn send_transmission_package_service(
        _tenant_id: &str,
        _election_id: &str,
        _area_id: &str,
        _tally_session_id: &str,
    ) -> anyhow::Result<()> {
        Err(anyhow::anyhow!(super::MIRU_DISABLED))
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
