// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Dedicated independent product staff enrollment listener.
//! No certificate issuance, ordinary API, signing/device routes or fallback.

use crate::routes::private_product_setup_ca::product_setup_ca;
use anyhow::{anyhow, ensure, Result};
use deadpool_postgres::{Manager, Pool, Runtime};
use serde::Deserialize;
use std::{
    fs::{self, File},
    io::Read,
    net::SocketAddr,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    time::Duration,
};
use tokio_postgres::NoTls;
use uuid::Uuid;

const PROFILE: &str = "sequent-operational-product-setup-ca-v1";
const MAX_FILE: u64 = 128 * 1024;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Installation {
    profile: String,
    tenant: Uuid,
    officer: Uuid,
    listen: SocketAddr,
    officer_spki_sha256: String,
    postgres_socket: PathBuf,
    postgres_database: String,
    setup_ca_db_login: String,
}

fn read(root: &Path, name: &str, private: bool) -> Result<Vec<u8>> {
    let path = root.join(name);
    let meta = fs::symlink_metadata(&path)?;
    let mode = meta.permissions().mode();
    ensure!(meta.is_file() && mode & 0o222 == 0 && (!private || mode & 0o007 == 0)
        && meta.len() > 0 && meta.len() <= MAX_FILE, "invalid protected CA installation file");
    let mut data = Vec::new();
    File::open(path)?.take(MAX_FILE + 1).read_to_end(&mut data)?;
    ensure!(!data.is_empty() && data.len() <= MAX_FILE as usize, "invalid protected CA installation file size");
    Ok(data)
}

fn role_name(value: &str) -> bool {
    !value.is_empty() && value.len() <= 63
        && value.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-')
}

fn configuration(root: &Path) -> Result<(Installation, [u8; 32])> {
    let meta = fs::symlink_metadata(root)?;
    ensure!(root.is_absolute() && root.canonicalize()?.as_path() == root
        && meta.is_dir() && meta.permissions().mode() & 0o222 == 0,
        "private setup CA requires a canonical immutable installation mount");
    let conf: Installation = serde_json::from_slice(&read(root, "setup-ca.json", false)?)
        .map_err(|_| anyhow!("invalid protected setup CA manifest"))?;
    ensure!(conf.profile == PROFILE && !conf.tenant.is_nil() && !conf.officer.is_nil()
        && !conf.listen.ip().is_unspecified() && conf.listen.port() != 0
        && role_name(&conf.postgres_database) && role_name(&conf.setup_ca_db_login)
        && conf.officer_spki_sha256.len() == 64
        && conf.officer_spki_sha256.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
        "invalid protected setup CA scope or role");
    let mut pin = [0u8; 32];
    hex::decode_to_slice(&conf.officer_spki_sha256, &mut pin)?;
    ensure!(pin != [0; 32], "empty setup CA device pin");
    let socket = fs::symlink_metadata(&conf.postgres_socket)?;
    ensure!(conf.postgres_socket.is_absolute()
        && conf.postgres_socket.canonicalize()? == conf.postgres_socket
        && socket.is_dir() && socket.permissions().mode() & 0o022 == 0,
        "setup CA requires a protected local PostgreSQL Unix socket");
    Ok((conf, pin))
}

async fn pool(conf: &Installation) -> Result<Pool> {
    let mut db = tokio_postgres::Config::new();
    db.host_path(&conf.postgres_socket).dbname(&conf.postgres_database).user(&conf.setup_ca_db_login)
        .connect_timeout(Duration::from_secs(4));
    let pool = Pool::builder(Manager::new(db, NoTls)).runtime(Runtime::Tokio1).max_size(4).build()?;
    let conn = pool.get().await.map_err(|_| anyhow!("private CA database unavailable"))?;
    let okay: bool = conn.query_one(
        "SELECT session_user::text=$1 AND current_user::text=$1 AND current_database()=$2",
        &[&conf.setup_ca_db_login, &conf.postgres_database],
    ).await.map_err(|_| anyhow!("private CA database role could not be verified"))?.try_get(0)?;
    ensure!(okay, "unexpected private CA database role");
    drop(conn);
    Ok(pool)
}

pub async fn rocket(root: &Path) -> Result<rocket::Rocket<rocket::Build>> {
    let (conf, officer_pin) = configuration(root)?;
    let cert = read(root, "setup-ca.chain.pem", false)?;
    let key = read(root, "setup-ca.key.pem", true)?;
    let accepted_officers = read(root, "setup-officer-clients.root.pem", false)?;
    let pg = pool(&conf).await?;
    product_setup_ca(conf.listen, conf.tenant, conf.officer, officer_pin, pg, &cert, &key, &accepted_officers)
        .await.map_err(|_| anyhow!("private setup CA mTLS configuration or database privileges refused"))
}

