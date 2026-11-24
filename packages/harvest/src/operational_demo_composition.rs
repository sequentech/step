// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Explicit installation point for the isolated SoftHSM or YubiHSM demo.

use crate::ports::database::DatabasePools;
use crate::routes::{
    operational_initialize::{self, DemoSigningScope, InitializeBroker},
    operational_initialize_broker::PrivateInitializeBroker,
    operational_initialize_setup::{self, SetupDispatcher},
    private_operational_ca,
};
use crate::services::{
    dependencies::HarvestServices,
    operational_portal_auth::OperationalPortalAuth,
};
use deadpool_postgres::{Manager, Pool, Runtime};
use jsonwebtoken::jwk::JwkSet;
use reqwest::{Certificate, Identity};
use rocket::config::Config;
use rocket::data::{Limits, ToByteUnit};
use rocket::http::Status;
use rocket::{Build, Rocket};
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::Read;
use std::net::{Ipv4Addr, SocketAddr};
use std::os::unix::fs::{FileTypeExt, MetadataExt};
use std::path::Path;
use std::sync::Arc;
use tokio_postgres::NoTls;
use uuid::Uuid;
use windmill::services::signing::{
    certificates::OpensslCertificateVerifier,
    directory::KeycloakUserDirectory,
    operational_device::{installed_demo, DemoInstallation as InstalledDemo},
    operational_policy,
};

const FIXTURE_IDP_BASE: &str = "https://127.0.0.1:18443";
const GOLD_CLIENT: &str = "sequent-operational-demo-14990";
const PORTAL_CLIENT: &str = "admin-portal";
const BROKER_ORIGIN: &str = "https://127.0.0.1:19443/";
const DEVICE_ORIGIN: &str = "https://127.0.0.1:20443/";
const PORTAL_API_PORT: u16 = 22443;
const PRIVATE_CA_PORT: u16 = 21443;

/// Private outbound Initialize broker transport; never reuse the setup identity.
struct BrokerTransport<'a> {
    pub roots: &'a [Certificate],
    pub identity: Identity,
}

/// Private outbound setup device transport, independent of the broker.
struct SetupTransport {
    pub root: Certificate,
    pub identity: Identity,
}

/// Private inbound CA assessment listener. Run it on isolated loopback
/// alongside its synthetic device client, never behind the browser ingress.
struct CaTransport<'a> {
    pub address: SocketAddr,
    pub device_peer_spki: [u8; 32],
    pub server_chain: &'a [u8],
    pub server_key: &'a [u8],
    pub device_client_ca: &'a [u8],
}

struct DemoInstallation<'a> {
    installation: InstalledDemo,
    /// Dedicated loopback-only API; the approved fixture ingress handles TLS.
    pub portal_address: SocketAddr,
    pub broker: BrokerTransport<'a>,
    pub setup: SetupTransport,
    pub ca: CaTransport<'a>,
    /// Verified, installation-local public JWKS from the exact synthetic realm.
    pub portal_jwks: JwkSet,
    directory_db: Pool,
    portal_db: Pool,
    pub ca_assessment_db: Pool,
}

/// Separate listeners. Launch as one isolated fixture, with failures in
/// either treated as a failed run; nothing is launched by this constructor.
struct DemoApplications {
    pub portal: Rocket<Build>,
    pub private_ca: Rocket<Build>,
}

fn validate_scope(
    tenant: Uuid,
    event: Uuid,
    portal: SocketAddr,
    ca: SocketAddr,
) -> Result<(), Status> {
    if tenant.is_nil()
        || event.is_nil()
        || portal != SocketAddr::from((Ipv4Addr::LOCALHOST, PORTAL_API_PORT))
        || ca != SocketAddr::from((Ipv4Addr::LOCALHOST, PRIVATE_CA_PORT))
    {
        return Err(Status::ServiceUnavailable);
    }
    Ok(())
}

async fn build_demo(
    config: DemoInstallation<'_>,
) -> Result<DemoApplications, Status> {
    let DemoInstallation {
        installation:
            InstalledDemo {
                tenant,
                event,
                device,
                run,
            },
        portal_address,
        broker,
        setup,
        ca,
        portal_jwks,
        directory_db,
        portal_db,
        ca_assessment_db,
    } = config;
    validate_scope(tenant, event, portal_address, ca.address)?;
    // Existing sealed demo DB marker and policy must pass before any routes
    // can be returned. Do not start probes, tasks or server listeners here.
    let mut services = HarvestServices::production();
    let connection = portal_db
        .get()
        .await
        .map_err(|_| Status::ServiceUnavailable)?;
    operational_policy::assert_startup_policy(&connection)
        .await
        .map_err(|_| Status::ServiceUnavailable)?;
    let sealed: Option<Uuid> = connection
        .query_one(
            "SELECT sequent_backend.operational_signing_demo_fixture_id($1,$2,$3,$4)",
            &[&tenant, &event, &"initialize-voting", &device.as_str()],
        )
        .await
        .map_err(|_| Status::ServiceUnavailable)?
        .try_get(0)
        .map_err(|_| Status::ServiceUnavailable)?;
    if sealed != Some(run) || run.is_nil() {
        return Err(Status::ServiceUnavailable);
    }
    drop(connection);
    let directory_db = Arc::new(directory_db);
    services.signing.verifier =
        Arc::new(OpensslCertificateVerifier::with_directory(Arc::new(
            KeycloakUserDirectory::on(directory_db.clone()),
        )));
    services.databases = Arc::new(FixturePools {
        portal: Arc::new(portal_db),
        directory: directory_db,
    });

    let portal_auth = OperationalPortalAuth::new(
        tenant,
        format!("{FIXTURE_IDP_BASE}/realms/tenant-{tenant}"),
        PORTAL_CLIENT.to_owned(),
        PORTAL_CLIENT.to_owned(),
        portal_jwks,
    )
    .ok_or(Status::ServiceUnavailable)?;
    let broker = PrivateInitializeBroker::new(
        tenant,
        BROKER_ORIGIN,
        FIXTURE_IDP_BASE,
        GOLD_CLIENT,
        broker.roots,
        broker.identity,
    )
    .map_err(|_| Status::ServiceUnavailable)?;
    let setup = SetupDispatcher::new(
        tenant,
        event,
        DEVICE_ORIGIN,
        setup.root,
        setup.identity,
    )
    .ok_or(Status::ServiceUnavailable)?;
    let private_ca = private_operational_ca::unmounted_authority(
        ca.address,
        tenant,
        ca.device_peer_spki,
        device,
        ca_assessment_db,
        ca.server_chain,
        ca.server_key,
        ca.device_client_ca,
    )
    .await?;
    // No generic routes, catch-alls, request logging fairing or test-provider
    // configuration on this listener. Do not attach it to normal Harvest.
    let portal = rocket::custom(Config {
        address: portal_address.ip(),
        port: portal_address.port(),
        limits: Limits::default().limit("json", 128.kibibytes()),
        ..Config::default()
    })
    .manage(services)
    .manage(portal_auth)
    .manage(InitializeBroker(Box::new(broker)))
    .manage(DemoSigningScope {
        tenant,
        event,
        run,
        profile: device.as_str(),
    })
    .manage(setup)
    .mount("/", portal_routes());
    Ok(DemoApplications { portal, private_ca })
}

fn portal_routes() -> Vec<rocket::Route> {
    rocket::routes![
        operational_initialize::status,
        operational_initialize::start,
        operational_initialize::finish,
        operational_initialize::cancel_unused,
        operational_initialize::reconcile,
        operational_initialize_setup::status,
        operational_initialize_setup::start,
    ]
}

const RUNTIME: &str = "/run/sequent-operational-demo-14990";
const APP_DB: &str = "operational_demo_14990";
const DIRECTORY_DB: &str = "operational_demo_keycloak_14990";
const MAX_PEM: usize = 64 * 1024;

struct FixturePools {
    portal: Arc<Pool>,
    directory: Arc<Pool>,
}

#[rocket::async_trait]
impl DatabasePools for FixturePools {
    async fn hasura(&self) -> Arc<Pool> {
        self.portal.clone()
    }
    async fn keycloak(&self) -> Arc<Pool> {
        self.directory.clone()
    }
}

fn invalid<T>() -> Result<T, Status> {
    Err(Status::ServiceUnavailable)
}

fn pinned_file(
    path: &Path,
    limit: usize,
    private: bool,
) -> Result<Vec<u8>, Status> {
    // The supervisor owns the immutable, read-only mount tree. Reject a
    // symlink, mutable file, wrong kind, oversized read, or changed inode.
    if path.canonicalize().ok().as_deref() != Some(path) {
        return invalid();
    }
    let before =
        fs::symlink_metadata(path).map_err(|_| Status::ServiceUnavailable)?;
    if !before.is_file()
        || before.mode() & 0o022 != 0
        || private && before.mode() & 0o077 != 0
        || before.len() > limit as u64
    {
        return invalid();
    }
    let handle = File::open(path).map_err(|_| Status::ServiceUnavailable)?;
    let same = |other: &fs::Metadata| {
        before.dev() == other.dev()
            && before.ino() == other.ino()
            && before.len() == other.len()
            && before.mode() == other.mode()
            && before.mtime() == other.mtime()
            && before.mtime_nsec() == other.mtime_nsec()
    };
    if !same(&handle.metadata().map_err(|_| Status::ServiceUnavailable)?) {
        return invalid();
    }
    let mut bytes = Vec::new();
    (&handle)
        .take((limit + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| Status::ServiceUnavailable)?;
    if bytes.is_empty()
        || bytes.len() > limit
        || !same(&handle.metadata().map_err(|_| Status::ServiceUnavailable)?)
    {
        return invalid();
    }
    Ok(bytes)
}

fn cert_pin(cert: &[u8]) -> Result<[u8; 32], Status> {
    let public = openssl::x509::X509::from_pem(cert)
        .and_then(|c| c.public_key())
        .and_then(|k| k.public_key_to_der())
        .map_err(|_| Status::ServiceUnavailable)?;
    Ok(Sha256::digest(public).into())
}

async fn fixed_pool(
    sock: &Path,
    database: &str,
    role: &str,
) -> Result<Pool, Status> {
    let mut config = tokio_postgres::Config::new();
    config
        .host_path(sock)
        .dbname(database)
        .user(role)
        .connect_timeout(std::time::Duration::from_secs(3));
    let pool = Pool::builder(Manager::new(config, NoTls))
        .runtime(Runtime::Tokio1)
        .max_size(4)
        .build()
        .map_err(|_| Status::ServiceUnavailable)?;
    let conn = pool.get().await.map_err(|_| Status::ServiceUnavailable)?;
    let expected: bool = conn
        .query_one(
            "SELECT current_user = $1::text AND session_user = $1::text
         AND current_database() = $2::text
         AND inet_client_addr() IS NULL
         AND current_setting('server_version_num')::integer >= 170000
         AND NOT (rolsuper OR rolcreaterole OR rolcreatedb OR rolreplication OR rolbypassrls)
         FROM pg_catalog.pg_roles WHERE rolname = current_user",
            &[&role, &database],
        )
        .await
        .map_err(|_| Status::ServiceUnavailable)?
        .try_get(0)
        .map_err(|_| Status::ServiceUnavailable)?;
    if !expected {
        return invalid();
    }
    let role_is_scoped = match role {
        "os14990_harvest" => conn.query_one(
            "SELECT pg_has_role(current_user,'sequent_os_approve','USAGE')
              AND NOT EXISTS (SELECT 1 FROM pg_roles r
                WHERE r.rolname IN ('sequent_os_setup_authority','sequent_os_setup_device',
                    'sequent_os_initialize_ca','sequent_os_initialize_device',
                    'sequent_os_initialize_orchestrator','sequent_os_orchestrator',
                    'sequent_os_device','sequent_os_ledger_api_owner',
                    'sequent_os_ledger_data_owner','sequent_os_recovery')
                  AND pg_has_role(current_user,r.oid,'MEMBER'))
              AND NOT has_table_privilege(current_user,
                'sequent_backend.operational_signing_intent','SELECT,INSERT,UPDATE,DELETE,TRUNCATE,TRIGGER')
              AND NOT has_any_column_privilege(current_user,
                'sequent_backend.operational_signing_intent','INSERT,UPDATE,REFERENCES')
              AND NOT EXISTS (SELECT 1 FROM pg_attribute a
                WHERE a.attrelid='sequent_backend.operational_signing_intent'::regclass
                  AND a.attnum>0 AND NOT a.attisdropped
                  AND a.attname NOT IN ('tenant_id','event_id','request_id','actor','phase',
                    'purpose','binding','receipt','reservation_id','applied_signature_sha256',
                    'applied_pdf_revision_id','applied_pdf_document_id','applied_pdf_sha256')
                  AND has_column_privilege(current_user,a.attrelid,a.attnum,'SELECT'))",
            &[],
        ).await,
        "os14990_harvest_directory" => conn.query_one(
            "WITH readable(relation,field) AS (VALUES
               ('realm','id'),('realm','name'),('user_entity','id'),('user_entity','username'),
               ('user_entity','first_name'),('user_entity','last_name'),('user_entity','realm_id'),
               ('user_entity','enabled'),('user_entity','service_account_client_link'),
               ('keycloak_role','id'),('keycloak_role','realm_id'),('keycloak_role','client_role'),
               ('keycloak_role','name'),('composite_role','composite'),('composite_role','child_role'),
               ('user_group_membership','user_id'),('user_group_membership','group_id'),
               ('keycloak_group','id'),('keycloak_group','parent_group'),('keycloak_group','realm_id'),
               ('user_role_mapping','role_id'),('user_role_mapping','user_id'),
               ('group_role_mapping','group_id'),('group_role_mapping','role_id'))
             SELECT NOT has_schema_privilege(current_user,'public','CREATE')
              AND NOT EXISTS (SELECT 1 FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace
                WHERE n.nspname='public' AND c.relkind IN ('r','p','v','m','f')
                  AND (has_table_privilege(current_user,c.oid,'INSERT,UPDATE,DELETE,TRUNCATE,TRIGGER')
                    OR has_any_column_privilege(current_user,c.oid,'INSERT,UPDATE,REFERENCES')
                    OR EXISTS (SELECT 1 FROM pg_attribute a
                         WHERE a.attrelid=c.oid AND a.attnum>0 AND NOT a.attisdropped
                           AND has_column_privilege(current_user,c.oid,a.attnum,'SELECT')
                           AND NOT EXISTS (SELECT 1 FROM readable
                              WHERE relation=c.relname AND field=a.attname))))
              AND NOT EXISTS (SELECT 1 FROM readable
              WHERE NOT has_column_privilege(current_user,'public.' || relation,field,'SELECT'))",
            &[],
        ).await,
        // Constructors independently verify narrowly scoped setup/CA grants.
        "os14990_ca" => conn.query_one("SELECT true", &[]).await,
        _ => return invalid(),
    }.map_err(|_| Status::ServiceUnavailable)?
        .try_get::<_, bool>(0).map_err(|_| Status::ServiceUnavailable)?;
    if !role_is_scoped {
        return invalid();
    }
    drop(conn);
    Ok(pool)
}

fn cert(pem: &[u8]) -> Result<Certificate, Status> {
    Certificate::from_pem(pem).map_err(|_| Status::ServiceUnavailable)
}
fn identity(certificate: &[u8], key: &[u8]) -> Result<Identity, Status> {
    Identity::from_pkcs8_pem(certificate, key)
        .map_err(|_| Status::ServiceUnavailable)
}

/// Explicit entrypoint for the dedicated demo binary; never called by
/// Harvest or an HSM wrapper. Its supervisor must fence the netns and mounts.
pub async fn serve_disposable() -> Result<(), Status> {
    // Before production service constructors can create a producer or open
    // a listener, require the exact closed queue/storage paths and live queue.
    windmill::app::check_isolated_signing_producer()
        .await
        .map_err(|_| Status::ServiceUnavailable)?;
    let runtime = Path::new(RUNTIME);
    let harvest = runtime.join("harvest");
    let sock = runtime.join("postgres");
    for dir in [runtime, harvest.as_path(), sock.as_path()] {
        let meta = fs::symlink_metadata(dir)
            .map_err(|_| Status::ServiceUnavailable)?;
        if !meta.is_dir()
            || meta.mode() & 0o022 != 0
            || dir.canonicalize().ok().as_deref() != Some(dir)
        {
            return invalid();
        }
    }
    if !fs::symlink_metadata(sock.join(".s.PGSQL.5432"))
        .map_err(|_| Status::ServiceUnavailable)?
        .file_type()
        .is_socket()
    {
        return invalid();
    }
    let installation = installed_demo().ok_or(Status::ServiceUnavailable)?;
    let pem = |name: &str, private| {
        pinned_file(&harvest.join(name), MAX_PEM, private)
    };
    let roots = [cert(&pem("broker-root.pem", false)?)?];
    let broker_leaf = pem("broker-client.pem", false)?;
    let setup_leaf = pem("setup-client.pem", false)?;
    if cert_pin(&broker_leaf)? == cert_pin(&setup_leaf)? {
        return invalid();
    }
    let mut setup_identity = setup_leaf;
    setup_identity.push(b'\n');
    setup_identity.extend(pem("setup-client-key.pem", true)?);
    let setup_identity = Identity::from_pem(&setup_identity)
        .map_err(|_| Status::ServiceUnavailable)?;
    let portal_jwks = serde_json::from_slice(&pinned_file(
        &harvest.join("portal.jwks.json"),
        MAX_PEM,
        false,
    )?)
    .map_err(|_| Status::ServiceUnavailable)?;
    let ca_pin = pinned_file(&harvest.join("ca-device-spki.hex"), 65, false)?;
    let pin = std::str::from_utf8(&ca_pin)
        .map_err(|_| Status::ServiceUnavailable)?
        .trim_end_matches('\n');
    if pin.len() != 64
        || !pin
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return invalid();
    }
    let mut peer_pin = [0u8; 32];
    hex::decode_to_slice(pin, &mut peer_pin)
        .map_err(|_| Status::ServiceUnavailable)?;
    let applications = build_demo(DemoInstallation {
        installation,
        portal_address: (Ipv4Addr::LOCALHOST, PORTAL_API_PORT).into(),
        portal_jwks,
        broker: BrokerTransport {
            roots: &roots,
            identity: identity(
                &broker_leaf,
                &pem("broker-client-key.pem", true)?,
            )?,
        },
        setup: SetupTransport {
            root: cert(&pem("setup-root.pem", false)?)?,
            identity: setup_identity,
        },
        ca: CaTransport {
            address: (Ipv4Addr::LOCALHOST, PRIVATE_CA_PORT).into(),
            device_peer_spki: peer_pin,
            server_chain: &pem("ca-server.pem", false)?,
            server_key: &pem("ca-server-key.pem", true)?,
            device_client_ca: &pem("ca-device-root.pem", false)?,
        },
        portal_db: fixed_pool(&sock, APP_DB, "os14990_harvest").await?,
        directory_db: fixed_pool(
            &sock,
            DIRECTORY_DB,
            "os14990_harvest_directory",
        )
        .await?,
        ca_assessment_db: fixed_pool(&sock, APP_DB, "os14990_ca").await?,
    })
    .await?;
    let portal = applications
        .portal
        .ignite()
        .await
        .map_err(|_| Status::ServiceUnavailable)?;
    let private_ca = applications
        .private_ca
        .ignite()
        .await
        .map_err(|_| Status::ServiceUnavailable)?;
    // Do not run half a fixture if either server stops or fails to bind.
    tokio::select! {
        _ = portal.launch() => invalid(),
        _ = private_ca.launch() => invalid(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refuses_nil_scope_and_non_loopback_or_wrong_ports() {
        let tenant = Uuid::from_u128(1);
        let event = Uuid::from_u128(2);
        let portal = "127.0.0.1:22443".parse().unwrap();
        let ca = "127.0.0.1:21443".parse().unwrap();
        assert!(validate_scope(tenant, event, portal, ca).is_ok());
        assert!(validate_scope(Uuid::nil(), event, portal, ca).is_err());
        assert!(validate_scope(tenant, Uuid::nil(), portal, ca).is_err());
        for address in ["0.0.0.0:21443", "127.0.0.1:0", "127.0.0.1:22443"] {
            assert!(validate_scope(
                tenant,
                event,
                portal,
                address.parse().unwrap()
            )
            .is_err());
        }
    }

    #[test]
    fn private_authority_never_leaks_into_the_dedicated_portal_routes() {
        let actual: std::collections::BTreeSet<_> = portal_routes()
            .into_iter()
            .map(|route| {
                (route.method.to_string(), route.uri.path().to_string())
            })
            .collect();
        let expected = [
            "status",
            "start",
            "finish",
            "cancel",
            "reconcile",
            "setup/status",
            "setup",
        ]
        .into_iter()
        .map(|path| {
            (
                "POST".to_owned(),
                format!("/operational-signing/initialize/{path}"),
            )
        })
        .collect();
        assert_eq!(actual, expected);
    }
}
