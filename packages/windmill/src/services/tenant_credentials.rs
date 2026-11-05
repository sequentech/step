// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Trustees and logins of a tenant created from the admin portal.

use crate::postgres::trustee::{copy_trustees_to_tenant, get_all_trustees};
use crate::services::ceremonies::encrypter::encrypt_file_inner;
use crate::services::documents::upload_and_return_document_with_annotations;
use crate::services::password::generate_random_string_with_charset;
use anyhow::{anyhow, bail, Context, Result};
use deadpool_postgres::Transaction;
use sequent_core::services::keycloak::{get_tenant_realm, KeycloakAdminClient};
use sequent_core::types::hasura::core::{DocumentAnnotations, Trustee};
use sequent_core::types::keycloak::User;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use tracing::instrument;
use uuid::Uuid;

/// A keys ceremony needs a threshold of at least 2.
pub const MIN_TRUSTEES: usize = 2;
const ADMIN_USERNAME: &str = "admin";
const TRUSTEE_GROUP: &str = "/trustee";
const LEAD_TRUSTEE_ROLES: [&str; 2] = ["trustee-read", "trustee-write"];
const FILE_PASSWORD_LENGTH: usize = 64;
const FILE_PASSWORD_CHARSET: &str =
    "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-_.";
const LOGIN_PASSWORD_LENGTH: usize = 24;
// No leading character here may start a spreadsheet formula
const LOGIN_PASSWORD_CHARSET: &str =
    "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_.!#%";
// Design open question: each owner sets their own password at first login
const TEMPORARY_PASSWORDS: bool = true;

/// What the operator asked for, sent to the worker in the task message.
#[derive(Serialize, Deserialize, Clone)]
pub struct TenantCredentialsRequest {
    pub num_trustees: usize,
    pub document_id: String,
    pub file_password: String,
}

impl TenantCredentialsRequest {
    pub fn new(num_trustees: usize) -> Self {
        Self {
            num_trustees,
            document_id: Uuid::new_v4().to_string(),
            file_password: generate_random_string_with_charset(
                FILE_PASSWORD_LENGTH,
                FILE_PASSWORD_CHARSET,
            ),
        }
    }
}

impl fmt::Debug for TenantCredentialsRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TenantCredentialsRequest")
            .field("num_trustees", &self.num_trustees)
            .field("document_id", &self.document_id)
            .finish_non_exhaustive()
    }
}

/// One line of the credentials file.
pub struct TenantLogin {
    role: &'static str,
    username: String,
    password: String,
}

fn super_admin_tenant_id() -> Result<String> {
    std::env::var("SUPER_ADMIN_TENANT_ID").context("Missing variable SUPER_ADMIN_TENANT_ID")
}

fn generate_login_password() -> String {
    generate_random_string_with_charset(LOGIN_PASSWORD_LENGTH, LOGIN_PASSWORD_CHARSET)
}

/// The first `num_trustees` usable trustees, in natural order (trustee2 before trustee10).
pub fn select_trustees(mut trustees: Vec<Trustee>, num_trustees: usize) -> Result<Vec<Trustee>> {
    trustees.retain(|trustee| trustee.name.is_some() && trustee.public_key.is_some());
    if num_trustees < MIN_TRUSTEES || num_trustees > trustees.len() {
        bail!(
            "The number of trustees must be between {MIN_TRUSTEES} and {}, not {num_trustees}",
            trustees.len()
        );
    }
    trustees.sort_by_key(|trustee| {
        let name = trustee.name.clone().unwrap_or_default();
        (name.len(), name)
    });
    trustees.truncate(num_trustees);
    Ok(trustees)
}

/// Reads and checks the trustees to copy, before anything is created.
#[instrument(skip(hasura_transaction), err)]
pub async fn select_source_trustees(
    hasura_transaction: &Transaction<'_>,
    num_trustees: usize,
) -> Result<Vec<Trustee>> {
    let trustees = get_all_trustees(hasura_transaction, &super_admin_tenant_id()?).await?;
    select_trustees(trustees, num_trustees)
}

pub fn credentials_csv(tenant_id: &str, slug: &str, logins: &[TenantLogin]) -> Result<Vec<u8>> {
    let mut writer = csv::Writer::from_writer(vec![]);
    writer.write_record(["tenant_slug", "tenant_id", "role", "username", "password"])?;
    for login in logins {
        writer.write_record([
            slug,
            tenant_id,
            login.role,
            login.username.as_str(),
            login.password.as_str(),
        ])?;
    }
    writer
        .into_inner()
        .map_err(|err| anyhow!("Error finishing the credentials file: {}", err.error()))
}

#[instrument(skip_all, err)]
async fn set_admin_password(client: &KeycloakAdminClient, realm: &str) -> Result<TenantLogin> {
    let user_id = client
        .get_user_id_by_username(realm, ADMIN_USERNAME)
        .await?
        .ok_or_else(|| anyhow!("The tenant realm has no {ADMIN_USERNAME} user"))?;
    let password = generate_login_password();
    client
        .set_user_password(realm, &user_id, &password, TEMPORARY_PASSWORDS)
        .await?;
    Ok(TenantLogin {
        role: "admin",
        username: ADMIN_USERNAME.to_string(),
        password,
    })
}

#[instrument(skip_all, err)]
async fn create_trustee_login(
    client: &KeycloakAdminClient,
    realm: &str,
    tenant_id: &str,
    name: &str,
    position: usize,
) -> Result<TenantLogin> {
    let user = User {
        id: None,
        attributes: None,
        email: Some(format!("{name}@sequent.local")),
        email_verified: Some(true),
        enabled: Some(true),
        first_name: Some("Trustee".to_string()),
        last_name: Some((position + 1).to_string()),
        username: Some(name.to_string()),
        area: None,
        votes_info: None,
    };
    let attributes = HashMap::from([
        ("trustee".to_string(), vec![name.to_string()]),
        ("tenant-id".to_string(), vec![tenant_id.to_string()]),
    ]);
    let created = client
        .create_user(
            realm,
            &user,
            Some(attributes),
            Some(vec![TRUSTEE_GROUP.to_string()]),
        )
        .await?;
    let user_id = created
        .id
        .ok_or_else(|| anyhow!("Keycloak returned trustee {name} without an id"))?;
    if position == 0 {
        client
            .add_realm_roles_to_user(realm, &user_id, &LEAD_TRUSTEE_ROLES)
            .await?;
    }
    let password = generate_login_password();
    client
        .set_user_password(realm, &user_id, &password, TEMPORARY_PASSWORDS)
        .await?;
    Ok(TenantLogin {
        role: "trustee",
        username: name.to_string(),
        password,
    })
}

#[instrument(skip_all, err)]
async fn store_credentials_file(
    hasura_transaction: &Transaction<'_>,
    owner_tenant_id: &str,
    slug: &str,
    request: &TenantCredentialsRequest,
    csv: &[u8],
) -> Result<()> {
    // Both files live in this directory, which goes away when it drops
    let directory = tempfile::tempdir().context("Error creating a temporary directory")?;
    let plain_path = directory.path().join("credentials.csv");
    std::fs::write(&plain_path, csv).context("Error writing the credentials file")?;
    let plain_path = plain_path.to_str().context("Invalid temporary file path")?;
    let encrypted_path = encrypt_file_inner(plain_path, &request.file_password)?;
    let size = std::fs::metadata(&encrypted_path)
        .context("Error reading the encrypted file size")?
        .len();

    upload_and_return_document_with_annotations(
        hasura_transaction,
        &encrypted_path,
        size,
        "application/octet-stream",
        owner_tenant_id,
        None,
        &format!("tenant-{slug}-credentials.csv.enc"),
        Some(request.document_id.clone()),
        false,
        &DocumentAnnotations::tenant_credentials(),
    )
    .await?;
    Ok(())
}

/// Copies the trustee rows, creates the logins and stores the encrypted file.
/// Returns the file's document id.
#[instrument(skip_all, err)]
pub async fn provision_tenant_credentials(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    slug: &str,
    trustees: &[Trustee],
    request: &TenantCredentialsRequest,
) -> Result<String> {
    let owner_tenant_id = super_admin_tenant_id()?;
    let trustee_ids: Vec<String> = trustees.iter().map(|trustee| trustee.id.clone()).collect();
    copy_trustees_to_tenant(
        hasura_transaction,
        &owner_tenant_id,
        tenant_id,
        &trustee_ids,
    )
    .await?;

    let realm = get_tenant_realm(tenant_id);
    let client = KeycloakAdminClient::new()
        .await
        .map_err(|err| anyhow!("Error obtaining keycloak admin client: {err}"))?;
    let mut logins = vec![set_admin_password(&client, &realm).await?];
    for (position, trustee) in trustees.iter().enumerate() {
        let name = trustee
            .name
            .as_deref()
            .context("Selected trustee has no name")?;
        logins.push(create_trustee_login(&client, &realm, tenant_id, name, position).await?);
    }

    let csv = credentials_csv(tenant_id, slug, &logins)?;
    store_credentials_file(hasura_transaction, &owner_tenant_id, slug, request, &csv).await?;
    Ok(request.document_id.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn trustee(name: Option<&str>, public_key: Option<&str>) -> Trustee {
        Trustee {
            id: Uuid::new_v4().to_string(),
            public_key: public_key.map(str::to_string),
            name: name.map(str::to_string),
            created_at: None,
            last_updated_at: None,
            labels: None,
            annotations: None,
            tenant_id: Uuid::new_v4().to_string(),
        }
    }

    fn names(trustees: &[Trustee]) -> Vec<String> {
        trustees
            .iter()
            .filter_map(|trustee| trustee.name.clone())
            .collect()
    }

    #[test]
    fn select_trustees_takes_the_first_ones_in_natural_order() {
        let trustees = vec![
            trustee(Some("trustee10"), Some("k10")),
            trustee(Some("trustee2"), Some("k2")),
            trustee(Some("trustee1"), Some("k1")),
        ];

        let selected = select_trustees(trustees, 2).unwrap();

        assert_eq!(names(&selected), ["trustee1", "trustee2"]);
    }

    #[test]
    fn select_trustees_skips_rows_a_ceremony_cannot_use() {
        let trustees = vec![
            trustee(Some("trustee1"), Some("k1")),
            trustee(Some("trustee2"), None),
            trustee(None, Some("k3")),
        ];

        assert!(select_trustees(trustees, 2).is_err());
    }

    #[test]
    fn select_trustees_rejects_a_number_out_of_range() {
        let trustees = || {
            vec![
                trustee(Some("trustee1"), Some("k1")),
                trustee(Some("trustee2"), Some("k2")),
            ]
        };

        assert!(select_trustees(trustees(), 1).is_err());
        assert!(select_trustees(trustees(), 3).is_err());
        assert!(select_trustees(trustees(), 2).is_ok());
    }

    #[test]
    fn credentials_csv_has_one_row_per_login() {
        let logins = [
            TenantLogin {
                role: "admin",
                username: "admin".to_string(),
                password: "first,value".to_string(),
            },
            TenantLogin {
                role: "trustee",
                username: "trustee1".to_string(),
                password: "second".to_string(),
            },
        ];

        let csv = String::from_utf8(credentials_csv("id", "slug", &logins).unwrap()).unwrap();

        assert_eq!(
            csv,
            "tenant_slug,tenant_id,role,username,password\n\
             slug,id,admin,admin,\"first,value\"\n\
             slug,id,trustee,trustee1,second\n"
        );
    }

    #[test]
    fn request_debug_output_hides_the_file_password() {
        let request = TenantCredentialsRequest::new(3);

        assert_eq!(request.file_password.len(), FILE_PASSWORD_LENGTH);
        assert!(!format!("{request:?}").contains(&request.file_password));
    }
}
