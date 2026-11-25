// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Private assessment process for an installed normal Felix3 Post.

mod transport;

use crate::routes::operational_initialize::local::{protected, ROOT};
use crate::routes::private_operational_ca::unmounted_authority;
use anyhow::{ensure, Context, Result};
use deadpool_postgres::{Manager, Pool, Runtime, Transaction};
use rocket::http::Status;
use serde::{de::DeserializeOwned, Deserialize};
use std::fs;
use std::net::{Ipv4Addr, SocketAddrV4};
use std::os::unix::fs::FileTypeExt;
use std::path::Path;
use std::time::Duration;
use tokio_postgres::NoTls;
use transport::{Pin, Transport};
use uuid::Uuid;
use windmill::services::signing::operational_ca::{
    DeviceCaProbe, RequiredCaDevice,
};
use zeroize::Zeroizing;

const LOGIN: &str = "felix3_local_initialize_ca";
const DEVICE: RequiredCaDevice = RequiredCaDevice::DisposableSoftHsm;
const CA_PORT: u16 = 21443;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Installation {
    tenant: Uuid,
    event: Uuid,
    post: Uuid,
    publication: Uuid,
    installation: Uuid,
    profile: String,
}

impl Installation {
    fn parse(bytes: &[u8]) -> Result<Self> {
        ensure!(
            (1..=4096).contains(&bytes.len()),
            "invalid installation descriptor size"
        );
        let install: Self = object(bytes)?;
        let ids = [
            install.tenant,
            install.event,
            install.post,
            install.publication,
            install.installation,
        ];
        ensure!(
            install.profile == DEVICE.as_str() && !ids.contains(&Uuid::nil()),
            "invalid local CA installation"
        );
        Ok(install)
    }

    fn accepts_reference(&self, probe: &DeviceCaProbe) -> bool {
        (probe.tenant, probe.event) == (self.tenant, self.event)
            && ![probe.request, probe.official, probe.challenge, probe.fence]
                .contains(&Uuid::nil())
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Registration {
    role: String,
    listen: SocketAddrV4,
    server_spki_sha256: Pin,
    server_ca_sha256: Pin,
    client_ca_sha256: Pin,
}

impl Registration {
    fn validate(&self, assigned: Ipv4Addr) -> Result<()> {
        ensure!(
            self.role == "felix3-local-initialize-ca",
            "incorrect CA role"
        );
        ensure!(
            assigned.is_private()
                && self.listen == SocketAddrV4::new(assigned, CA_PORT),
            "expected the installed device-facing IPv4 address on port 21443"
        );
        Ok(())
    }
}

fn file(path: &Path, private: bool) -> Result<Vec<u8>> {
    protected(path, private).context("missing or unsafe local CA input")
}

fn json<T: DeserializeOwned>(path: &Path) -> Result<T> {
    object(&file(path, false)?)
}

fn object<T: DeserializeOwned>(bytes: &[u8]) -> Result<T> {
    let first = bytes.iter().copied().find(|b| !b.is_ascii_whitespace());
    ensure!(first == Some(b'{'), "local CA descriptor must be an object");
    serde_json::from_slice(bytes).context("invalid local CA descriptor")
}

fn read_only_mount(info: &str, target: &Path) -> bool {
    let mut found = false;
    for line in info.lines() {
        // These fixed container mount points contain no mountinfo escapes.
        let mut fields = line.split_ascii_whitespace().skip(4);
        let (Some(path), Some(options)) = (fields.next(), fields.next()) else {
            return false;
        };
        let path = Path::new(path);
        if path.starts_with(target) {
            if !options.split(',').any(|option| option == "ro") {
                return false;
            }
            found |= path == target;
        }
    }
    found
}

fn verify_mounts(root: &Path) -> Result<()> {
    let info = fs::read_to_string("/proc/self/mountinfo")?;
    for name in ["installation.json", "ca", "postgres"] {
        let path = root.join(name);
        ensure!(
            path.canonicalize()? == path && read_only_mount(&info, &path),
            "missing or writable CA installation mount"
        );
    }
    for entry in fs::read_dir(root)? {
        let name = entry?.file_name();
        ensure!(
            matches!(
                name.to_str(),
                Some("ca" | "installation.json" | "postgres")
            ),
            "unexpected sibling in CA installation"
        );
    }
    let socket = fs::symlink_metadata(root.join("postgres/.s.PGSQL.5432"))?;
    ensure!(
        socket.file_type().is_socket(),
        "missing app database socket"
    );
    Ok(())
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Database {
    database: String,
}

pub(crate) struct Scope {
    install: Installation,
    database: String,
}

impl Scope {
    async fn principal(&self, tx: &Transaction<'_>) -> Result<()> {
        let row = tx
            .query_one(include_str!("principal.sql"), &[&LOGIN, &self.database])
            .await?;
        let valid: bool = row.try_get(0)?;
        ensure!(valid, "unexpected CA database privileges");
        Ok(())
    }

    async fn pool(&self, root: &Path) -> Result<Pool> {
        ensure!(
            !self.database.is_empty() && self.database.len() <= 63,
            "invalid CA database name"
        );
        let password =
            Zeroizing::new(file(&root.join("ca/database-password"), true)?);
        let password = password.strip_suffix(b"\n").unwrap_or(&password);
        ensure!(
            !password.is_empty()
                && !password.iter().any(|b| matches!(*b, 0 | b'\r' | b'\n')),
            "invalid CA database password file"
        );
        let mut config = tokio_postgres::Config::new();
        config
            .host_path(&root.join("postgres"))
            .port(5432)
            .user(LOGIN)
            .dbname(&self.database)
            .password(password)
            .connect_timeout(Duration::from_secs(3))
            .options(
                "-c search_path=pg_catalog -c default_transaction_read_only=on",
            );
        let pool = Pool::builder(Manager::new(config, NoTls))
            .runtime(Runtime::Tokio1)
            .max_size(4)
            .build()?;
        let mut conn = pool.get().await?;
        let tx = conn.transaction().await?;
        self.principal(&tx).await?;
        let i = &self.install;
        let marker: Option<Uuid> = tx.query_one(
            "SELECT sequent_backend.operational_signing_local_initialize_installation_id(
                 $1::uuid,$2::uuid,$3::uuid,$4::uuid,$5::text)",
            &[&i.tenant, &i.event, &i.post, &i.publication, &DEVICE.as_str()],
        ).await?.try_get(0)?;
        ensure!(
            marker == Some(i.installation),
            "local CA installation marker mismatch"
        );
        tx.commit().await?;
        Ok(pool)
    }

    pub(crate) async fn authorize(
        &self,
        tx: &Transaction<'_>,
        probe: &DeviceCaProbe,
    ) -> Result<(), Status> {
        let i = &self.install;
        if !i.accepts_reference(probe) {
            return Err(Status::Forbidden);
        }
        self.principal(tx)
            .await
            .map_err(|_| Status::ServiceUnavailable)?;
        // The helper reads the marker and the reserved canonical request in
        // one statement. SQL NULL (withdrawn marker) and false both refuse.
        let allowed: Option<bool> = tx.query_one(
            "SELECT sequent_backend.felix3_initialize_ca_request_matches(
                 $1::uuid,$2::uuid,$3::uuid,$4::uuid,$5::uuid,$6::uuid,$7::uuid,$8::uuid,$9::uuid)",
            &[&i.tenant, &i.event, &i.post, &i.publication, &i.installation,
              &probe.request, &probe.official, &probe.challenge, &probe.fence],
        ).await.and_then(|row| row.try_get(0)).map_err(|_| Status::ServiceUnavailable)?;
        if allowed != Some(true) {
            return Err(Status::Forbidden);
        }
        Ok(())
    }
}

/// Called only by the dedicated assessment executable.
pub async fn serve() -> Result<()> {
    let root = Path::new(ROOT);
    verify_mounts(root).context("CA role mounts refused")?;
    let dir = root.join("ca");
    let install =
        Installation::parse(&file(&root.join("installation.json"), false)?)?;
    let registration: Registration = json(&dir.join("registration.json"))?;
    let assigned = std::env::var("FELIX3_CA_LISTEN_IP")?
        .parse()
        .context("invalid CA bridge IP")?;
    registration.validate(assigned)?;
    let tls = Transport::read(&dir, &registration)
        .context("CA transport registration refused")?;
    let Database { database } = json(&dir.join("database.json"))?;
    let scope = Scope { install, database };
    let db = scope
        .pool(root)
        .await
        .context("CA principal or installation refused")?;
    let app = unmounted_authority(
        registration.listen.into(),
        scope.install.tenant,
        tls.device_spki,
        DEVICE,
        db,
        &tls.chain,
        &tls.key,
        &tls.client_ca,
        scope,
    )
    .await
    .map_err(|_| anyhow::anyhow!("private assessment listener refused"))?;
    app.launch().await.map_err(|error| {
        let _ = error.kind();
        anyhow::anyhow!("private assessment listener failed")
    })?;
    Ok(())
}

#[cfg(test)]
mod tests;
