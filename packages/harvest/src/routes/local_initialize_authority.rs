// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Private eligibility for one normally imported and published synthetic Post.

use super::{
    signing_failure, HarvestServices, OperationalInitializeBroker, SigningCaller, SigningError,
};
use crate::routes::operational_initialize_broker::PrivateInitializeBroker;
use crate::services::operational_portal_auth::OperationalPortalAuth;
use jsonwebtoken::jwk::JwkSet;
use rocket::{Build, Request, Rocket};
use serde::Deserialize;
use std::fs;
use std::io::Read;
use std::net::{IpAddr, Ipv4Addr, SocketAddrV4};
use std::os::unix::fs::MetadataExt;
use std::path::Path;
use deadpool_postgres::Transaction;
use sequent_core::services::keycloak::get_tenant_realm;
use sequent_core::signing::{InitializeVotingSubject, SigningAction};
use uuid::Uuid;
use windmill::postgres::signing::get_signing_post;
use windmill::postgres::signing_actions::published_publication_of_post;
use windmill::services::signing::operational_initialize::InitializeSigningScope;
use windmill::services::signing::requests::read_request;
use windmill::services::signing::signers::{list_signers, GroupChange, Signer};

fn current_signer(
    signers: &[Signer],
    actor: &str,
    current_label: Option<&str>,
    request_label: Option<&str>,
) -> bool {
    signers.iter().any(|signer| {
        signer.user_id == actor
            && signer.eligible_for(current_label)
            && signer.eligible_for(request_label)
    })
}

/// Installed after import and publication, never deserialized from a request.
pub(crate) struct LocalInitializePost {
    pub(super) installation: Uuid,
    pub(super) tenant: Uuid,
    pub(super) event: Uuid,
    pub(super) post: Uuid,
    pub(super) publication: Uuid,
    pub(super) profile: &'static str,
    pub(super) authorization: reqwest::Url,
    pub(super) approval_client: String,
}

impl LocalInitializePost {
    async fn present(&self, tx: &Transaction<'_>) -> Result<bool, tokio_postgres::Error> {
        let marker: Option<Uuid> = tx.query_one(
            "SELECT sequent_backend.operational_signing_local_initialize_installation_id($1,$2,$3,$4,$5)",
            &[&self.tenant, &self.event, &self.post, &self.publication, &self.profile],
        ).await?.try_get(0)?;
        Ok(!self.installation.is_nil() && marker == Some(self.installation))
    }

    fn current_publication(&self, current: Option<&str>, signed: Option<&str>) -> bool {
        current == signed && current == Some(self.publication.to_string().as_str())
    }

    pub(super) fn permits_broker(&self, broker: &dyn OperationalInitializeBroker) -> bool {
        broker.authorization_endpoint() == &self.authorization
            && broker.oidc_client_id() == self.approval_client
    }

    pub(super) async fn verify(
        &self,
        app: &Transaction<'_>,
        services: &HarvestServices,
        actor: &SigningCaller,
        scope: &InitializeSigningScope,
    ) -> Result<(), SigningError> {
        let refused = || SigningError::forbidden(
            "Initialize approval is unavailable for this request.",
        );
        let tenant = scope.tenant();
        if tenant != self.tenant || scope.event() != self.event
            || scope.election() != Some(self.post) || scope.actor().to_string() != actor.user_id {
            return Err(refused());
        }
        if !self.present(app).await.map_err(SigningError::internal)? {
            return Err(refused());
        }
        let published = published_publication_of_post(app, tenant, self.event, self.post)
            .await.map_err(SigningError::internal)?.map(|id| id.to_string());
        let row = read_request(app, tenant, scope.request()).await.map_err(signing_failure)?;
        let subject: InitializeVotingSubject =
            serde_json::from_value(row.subject).map_err(|_| refused())?;
        if !self.current_publication(published.as_deref(), subject.publication_id.as_deref()) {
            return Err(refused());
        }
        let post = get_signing_post(app, tenant, self.event, self.post)
            .await
            .map_err(SigningError::internal)?
            .ok_or_else(refused)?;
        let label = post.permission_label.as_deref();
        let realm = get_tenant_realm(&tenant.to_string());
        let mut db = services.databases.keycloak().await.get().await.map_err(SigningError::internal)?;
        let directory = db.transaction().await.map_err(SigningError::internal)?;
        let signers = list_signers(
            &directory,
            &realm,
            SigningAction::InitializeVoting,
            &GroupChange::default(),
        )
        .await
        .map_err(SigningError::internal)?;
        if !actor.signs_for(label)
            || !current_signer(&signers, &actor.user_id, label, row.permission_label.as_deref())
        {
            return Err(refused());
        }
        directory.commit().await.map_err(SigningError::internal)?;
        Ok(())
    }
}

const ROOT: &str = "/run/sequent-felix3-local-initialize";

/// Only the Hasura connection's real TCP peer may use the local route. HTTP
/// headers are additional constraints and never establish the peer identity.
pub(crate) struct LocalIngress {
    peer: Ipv4Addr,
    authority: String,
}

impl LocalIngress {
    pub(crate) fn accepts(&self, request: &Request<'_>) -> bool {
        self.accepts_headers(request.remote().map(|peer| peer.ip()), request.headers())
    }

    fn accepts_headers(&self, peer: Option<IpAddr>, headers: &rocket::http::HeaderMap<'_>) -> bool {
        peer == Some(IpAddr::V4(self.peer))
            && headers.get("Host").eq(std::iter::once(self.authority.as_str()))
            && (headers.get("Origin").next().is_none()
                || headers.get("Origin").eq(std::iter::once("http://localhost:3002")))
            && headers.get("Cookie").next().is_none()
    }
}

// The shared descriptor names one installed Post. Each component reads its
// own pins from its private read-only mount after that installation exists.
#[derive(Deserialize)]
struct Descriptor {
    installation_id: Uuid,
    tenant_id: Uuid,
    election_event_id: Uuid,
    election_id: Uuid,
    publication_id: Uuid,
    device_profile: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct HarvestPins {
    portal_origin: String,
    portal_keycloak_base: String,
    portal_client: String,
    registered_issuer: String,
    registered_portal_client: String,
    registered_audience: String,
    authorization_endpoint: String,
    registered_callback: String,
    approval_client: String,
    listen: SocketAddrV4,
    hasura_peer: Ipv4Addr,
    handler_host: String,
    broker_origin: String,
    broker_peer: SocketAddrV4,
}

fn protected(path: &Path, private: bool) -> Option<Vec<u8>> {
    // All arguments are fixed component paths. Every ancestor up to / is
    // trusted before opening the leaf. Check the actual opened inode too.
    for parent in path.ancestors().skip(1) {
        let meta = fs::symlink_metadata(parent).ok()?;
        if !meta.is_dir() || meta.uid() != 0 || meta.mode() & 0o022 != 0 {
            return None;
        }
    }
    let meta = fs::symlink_metadata(path).ok()?;
    if !meta.is_file() || meta.uid() != 0 || meta.mode() & 0o222 != 0
        || (private && meta.mode() & 0o007 != 0)
    {
        return None;
    }
    let file = fs::File::open(path).ok()?;
    let actual = file.metadata().ok()?;
    if (actual.dev(), actual.ino()) != (meta.dev(), meta.ino()) || actual.len() > 65536 {
        return None;
    }
    let mut bytes = Vec::new();
    file.take(65537).read_to_end(&mut bytes).ok()?;
    (!bytes.is_empty() && bytes.len() <= 65536).then_some(bytes)
}

struct Installation {
    auth: OperationalPortalAuth,
    ingress: LocalIngress,
    post: LocalInitializePost,
    broker: PrivateInitializeBroker,
}

fn load(application: &Rocket<Build>, root: &Path) -> Option<Installation> {
    let file = |name: &str, private| protected(&root.join(name), private);
    let descriptor: Descriptor = serde_json::from_slice(&file("installation.json", false)?).ok()?;
    let pins: HarvestPins = serde_json::from_slice(&file("harvest/registration.json", false)?).ok()?;
    let profile = match descriptor.device_profile.as_str() {
        "yubihsm" => "yubihsm",
        "softhsm-test" => "softhsm-test",
        _ => return None,
    };
    if [descriptor.installation_id, descriptor.tenant_id, descriptor.election_event_id, descriptor.election_id, descriptor.publication_id]
        .contains(&Uuid::nil())
        || pins.registered_audience != pins.registered_portal_client
        || !pins.listen.ip().is_private() || pins.listen.port() == 0 || !pins.hasura_peer.is_private()
        || pins.hasura_peer == *pins.listen.ip()
        || pins.handler_host.is_empty() || pins.handler_host.len() > 63
        || pins.handler_host.starts_with('-') || pins.handler_host.ends_with('-')
        || !pins.handler_host.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    {
        return None;
    }
    let address = application.figment().extract_inner::<Ipv4Addr>("address").ok()?;
    let port = application.figment().extract_inner::<u16>("port").ok()?;
    if (address, port) != (*pins.listen.ip(), pins.listen.port()) {
        return None;
    }
    let keys: JwkSet = serde_json::from_slice(&file("harvest/realm-jwks.json", false)?).ok()?;
    let auth = OperationalPortalAuth::for_local_initialize(
        descriptor.tenant_id, &pins.portal_origin, &pins.portal_keycloak_base, &pins.portal_client,
        pins.registered_issuer, pins.registered_portal_client, keys,
    )?;
    let root_cert = reqwest::Certificate::from_pem(&file("harvest/broker-ca.pem", false)?).ok()?;
    let client_cert = file("harvest/broker-client.pem", false)?;
    let mut client_key = file("harvest/broker-client-key.pem", true)?;
    let identity = reqwest::Identity::from_pkcs8_pem(&client_cert, &client_key);
    client_key.fill(0);
    let broker = PrivateInitializeBroker::for_local_initialize(
        &auth, &pins.authorization_endpoint, &pins.registered_callback, &pins.approval_client,
        &pins.broker_origin, pins.broker_peer, &[root_cert], identity.ok()?,
    ).ok()?;
    let post = LocalInitializePost {
        installation: descriptor.installation_id,
        tenant: descriptor.tenant_id,
        event: descriptor.election_event_id,
        post: descriptor.election_id,
        publication: descriptor.publication_id,
        profile,
        authorization: broker.authorization_endpoint().clone(),
        approval_client: pins.approval_client,
    };
    let ingress = LocalIngress {
        peer: pins.hasura_peer,
        authority: format!("{}:{port}", pins.handler_host),
    };
    Some(Installation { auth, broker, post, ingress })
}

pub(crate) async fn mount(application: Rocket<Build>) -> Rocket<Build> {
    let Some(install) = load(&application, Path::new(ROOT)) else {
        return application;
    };
    let Some(services) = application.state::<HarvestServices>() else {
        return application;
    };
    let present = async {
        let mut db = services.databases.hasura().await.get().await.ok()?;
        let tx = db.transaction().await.ok()?;
        install.post.present(&tx).await.ok()
    }.await;
    if present != Some(true) {
        return application;
    }
    application
        .manage(install.auth)
        .manage(install.ingress)
        .manage(install.post)
        .manage(super::InitializeBroker(Box::new(install.broker)))
        .mount("/", rocket::routes![super::status, super::start, super::finish,
            super::cancel_unused, super::reconcile])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ingress_requires_the_actual_peer_and_exact_authority() {
        use rocket::http::HeaderMap;
        let ingress = LocalIngress { peer: Ipv4Addr::new(10, 0, 0, 17), authority: "harvest:8899".into() };
        let peer = Some(IpAddr::V4(ingress.peer));
        let mut valid = HeaderMap::new();
        valid.add_raw("Host", "harvest:8899");
        valid.add_raw("Origin", "http://localhost:3002");
        assert!(ingress.accepts_headers(peer, &valid));
        valid.add_raw("X-Forwarded-For", "10.0.0.17");
        assert!(!ingress.accepts_headers(None, &valid));
        assert!(!ingress.accepts_headers(Some(IpAddr::V4(Ipv4Addr::LOCALHOST)), &valid));
        for (name, value) in [
            ("Host", "harvest:8898"), ("Origin", "http://127.0.0.1:3002"), ("Cookie", "opaque"),
        ] {
            let mut wrong = valid.clone();
            wrong.replace_raw(name, value);
            assert!(!ingress.accepts_headers(peer, &wrong));
        }
        valid.add_raw("Host", "harvest:8899");
        assert!(!ingress.accepts_headers(peer, &valid));
    }

    #[test]
    fn local_post_requires_the_same_current_and_signed_publication() {
        let pin = LocalInitializePost {
            installation: Uuid::new_v4(), tenant: Uuid::new_v4(), event: Uuid::new_v4(),
            post: Uuid::new_v4(), publication: Uuid::new_v4(), profile: "yubihsm",
            authorization: reqwest::Url::parse("http://localhost:18090/").unwrap(),
            approval_client: "test-gold".into(),
        };
        let signed = pin.publication.to_string();
        assert!(pin.current_publication(Some(&signed), Some(&signed)));
        assert!(!pin.current_publication(Some(&Uuid::new_v4().to_string()), Some(&signed)));
        assert!(!pin.current_publication(None, Some(&signed)));
        assert!(!pin.current_publication(Some(&signed), Some(&Uuid::new_v4().to_string())));
        assert!(!pin.current_publication(Some(&signed), None));
    }

    #[test]
    fn same_enabled_signer_must_reach_the_frozen_and_current_post_labels() {
        let actor = Uuid::new_v4().to_string();
        let official = Signer {
            user_id: actor.clone(),
            username: "synthetic-signer".into(),
            first_name: None,
            last_name: None,
            title: None,
            labels: vec!["post-a".into()],
            groups: vec![],
        };
        let roster = [official];
        let label = Some("post-a");
        assert!(current_signer(&roster, &actor, label, label));
        assert!(!current_signer(&roster, &actor, Some("post-b"), label));
        assert!(!current_signer(&roster, &actor, label, Some("post-b")));
        assert!(!current_signer(&roster, &Uuid::new_v4().to_string(), label, label));
        assert!(!current_signer(&[], &actor, label, label));
    }
}
