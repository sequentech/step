// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The check before an election event import reads anything: a signed
//! configuration package is verified against the tenant's trust, its
//! revision must be newer than the last one imported, and only then is its
//! importable archive handed to the importer. An unsigned file is refused
//! where the tenant requires signatures.

use std::io::Write;

use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Transaction;
use sequent_core::election_config::package_verify::{
    admit, already_imported, crls_from_pem, freshness, importable_member, Admission,
    ConfigurationSigning, Freshness, VerifiedPackage, CONFIGURATION_SIGNING_SETTING,
};
use sequent_core::election_config::Rejected;
use tempfile::NamedTempFile;
use tracing::{info, instrument};

use crate::postgres::configuration_packages;
use crate::postgres::tenant::get_tenant_by_id;
use crate::services::electoral_log::{ElectoralLog, ElectoralLogAdminContext};
use crate::services::import::rejection::reject;
use crate::services::protocol_manager::get_event_board;
use electoral_log::messages::newtypes::{
    ConfigurationDesignDigest, ConfigurationPackageAction, ConfigurationPackageDetails,
};

const WHAT: &str = "configuration package";

/// The tenant's `configuration_signing` setting, if it has one.
pub async fn tenant_setting(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
) -> Result<Option<ConfigurationSigning>> {
    let tenant = get_tenant_by_id(hasura_transaction, tenant_id).await?;
    tenant
        .settings
        .as_ref()
        .and_then(|settings| settings.get(CONFIGURATION_SIGNING_SETTING))
        .map(|value| {
            serde_json::from_value(value.clone()).with_context(|| {
                format!("the tenant's {CONFIGURATION_SIGNING_SETTING} is malformed")
            })
        })
        .transpose()
}

/// Admits the file at `temp_file`: returns the file the importer reads (the
/// package's importable archive, for a package) and the verified package.
#[instrument(skip(hasura_transaction, temp_file), err)]
pub async fn admit_document(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    temp_file: NamedTempFile,
) -> Result<(NamedTempFile, Option<Box<VerifiedPackage>>)> {
    let bytes = std::fs::read(temp_file.path()).context("could not read the uploaded file")?;
    let setting = tenant_setting(hasura_transaction, tenant_id).await?;
    let lists = configuration_packages::revocation_lists(hasura_transaction, tenant_id).await?;

    let verified = match admit(&bytes, setting.as_ref(), lists) {
        Ok(Admission::Unsigned) => return Ok((temp_file, None)),
        Ok(Admission::Verified(verified)) => verified,
        Err(report) => return Err(anyhow::Error::new(Rejected::new(WHAT, report))),
    };

    let configuration = &verified.manifest.configuration;
    let last = configuration_packages::last_import(
        hasura_transaction,
        tenant_id,
        &configuration.external_id,
    )
    .await?;
    match freshness(&verified, last.as_ref()) {
        Ok(Freshness::New) => {}
        Ok(Freshness::AlreadyImported) => {
            return Err(reject(WHAT, already_imported(configuration.revision)));
        }
        Err(problem) => return Err(reject(WHAT, problem)),
    }

    let archive = importable_member(&verified).map_err(|problem| reject(WHAT, problem))?;
    let mut importable = NamedTempFile::new().context("could not create a temporary file")?;
    importable.write_all(archive)?;
    importable.flush()?;
    info!(
        external_id = %configuration.external_id,
        revision = configuration.revision,
        manifest_sha256 = %verified.manifest_sha256,
        "verified configuration package"
    );
    Ok((importable, Some(verified)))
}

/// The electoral log entry of an import, posted to the new event's board.
pub async fn log_import(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    package: &VerifiedPackage,
    initiator: Option<&ElectoralLogAdminContext>,
) -> Result<()> {
    let slug = std::env::var("ENV_SLUG").context("missing env var ENV_SLUG")?;
    let board_name = get_event_board(tenant_id, election_event_id, &slug);
    let electoral_log = ElectoralLog::new(
        hasura_transaction,
        tenant_id,
        Some(election_event_id),
        board_name.as_str(),
    )
    .await?;
    electoral_log
        .post_configuration_package(
            election_event_id.to_string(),
            ConfigurationPackageDetails {
                action: ConfigurationPackageAction::Imported,
                external_id: package.manifest.configuration.external_id.clone(),
                revision: package.manifest.configuration.revision,
                manifest_sha256: package.manifest_sha256.clone(),
                ballot_publication_id: None,
                design_digests: package
                    .manifest
                    .content
                    .ballot_designs
                    .iter()
                    .map(|design| ConfigurationDesignDigest {
                        area: design.area.clone(),
                        election: design.election.clone(),
                        sha256: design.sha256.clone(),
                    })
                    .collect(),
            },
            initiator.map(|initiator| initiator.user_id.clone()),
            initiator.and_then(|initiator| initiator.username.clone()),
        )
        .await
}

/// Records an imported package and keeps the revocation lists it carried,
/// so a revoked key's packages are refused from now on.
pub async fn record(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    package: &VerifiedPackage,
) -> Result<()> {
    configuration_packages::record_import(
        hasura_transaction,
        tenant_id,
        election_event_id,
        package,
    )
    .await?;
    let mut lists = Vec::new();
    for pem in &package.manifest.revocation_lists {
        lists.extend(crls_from_pem(pem).map_err(|error| anyhow!(error))?);
    }
    configuration_packages::remember_revocation_lists(hasura_transaction, tenant_id, &lists).await
}
